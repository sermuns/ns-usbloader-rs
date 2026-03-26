use egui::{Align, Checkbox, Layout, ProgressBar};
use egui_extras::{Column, TableBuilder};
use egui_toast::{ToastKind, Toasts};
use ironfoil_core::{GAME_BACKUP_EXTENSIONS, perform_usb_install};
use log::{error, info};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use crate::app::add_toast;
use crate::tabs::{OngoingInstallation, Pick, StagedFiles, stage_picked};

pub fn show(
    ui: &mut egui::Ui,
    recurse: &mut bool,
    for_sphaira: &mut bool,
    staged_files: &mut StagedFiles,
    maybe_ongoing_installation: &mut Option<OngoingInstallation>,
    toasts: &mut Toasts,
) {
    ui.horizontal(|ui| {
        if ui.button("💾 Pick file").clicked()
            && let Some(game_backup_path) = rfd::FileDialog::new()
                .add_filter("*", &GAME_BACKUP_EXTENSIONS)
                .pick_file()
        {
            stage_picked(Pick::File(game_backup_path), staged_files, toasts);
        }
        ui.label("or");
        #[cfg(target_os = "windows")]
        const PICK_DIRECTORY_LABEL: &str = "🗁 Pick folder";
        #[cfg(not(target_os = "windows"))]
        const PICK_DIRECTORY_LABEL: &str = "🗁 Pick directory";
        if ui.button(PICK_DIRECTORY_LABEL).clicked()
            && let Some(game_backup_path) = rfd::FileDialog::new().pick_folder()
        {
            stage_picked(
                Pick::Folder {
                    path: game_backup_path,
                    recurse: *recurse,
                },
                staged_files,
                toasts,
            );
        }
        ui.checkbox(recurse, "recurse?").on_hover_text(
            "Also discover game backups from subdirectories of the picked directory",
        );

        // FIXME: actually align right
        ui.add_space(16.);
        ui.checkbox(for_sphaira, "For Sphaira?");
    });

    ui.group(|ui| {
        if staged_files.is_empty() {
            ui.set_min_size(ui.available_size());
            ui.weak("No files staged. Pick using the buttons above!");
            return;
        }

        // FIXME: so fucking stupid... DONT USE HARDCODE FOR HEIGHT OF OTHER SHIT
        ui.set_height(ui.available_height() - 18. * 2.);

        if staged_files.has_any_selected() {
            if ui.button("Deselect all").clicked() {
                staged_files.deselect_all();
            }
        } else if ui.button("Select all").clicked() {
            staged_files.select_all();
        }

        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::remainder())
            .column(Column::auto())
            .header(0., |mut header| {
                header.col(|ui| {
                    ui.strong("Selected");
                });
                header.col(|ui| {
                    ui.strong("File name");
                });
                header.col(|ui| {
                    ui.strong("Size");
                });
            })
            .body(|body| {
                body.rows(18., staged_files.count(), |mut row| {
                    let staged_file = &mut staged_files.files[row.index()];
                    row.col(|ui| {
                        ui.add(Checkbox::without_text(&mut staged_file.selected));
                    });
                    row.col(|ui| {
                        ui.label(staged_file.path.file_name().unwrap().to_str().unwrap());
                    });
                    row.col(|ui| {
                        ui.label(humansize::format_size(
                            staged_file.file_size,
                            humansize::BINARY,
                        ));
                    });
                })
            });
    });
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if let Some(ongoing_installation) = maybe_ongoing_installation {
            if let Ok(progress_len) = ongoing_installation.progress_len_rx.try_recv() {
                info!("got progress len: {}", progress_len);
                ongoing_installation.last_progress_len = progress_len;
            }
            if let Ok(progress) = ongoing_installation.progress_rx.try_recv() {
                info!("got progress: {}", progress);
                ongoing_installation.last_progress = progress;
            }
            let progress: f32 = ongoing_installation.last_progress as f32
                / ongoing_installation.last_progress_len as f32;
            info!(
                "progress: {}/{} ({:.2}%)",
                ongoing_installation.last_progress,
                ongoing_installation.last_progress_len,
                progress * 100.
            );
            ui.horizontal(|ui| {
                if ui.button("❌ cancel").clicked() {
                    ongoing_installation.cancel.store(true, Ordering::Relaxed);
                }
                ui.add(ProgressBar::new(progress));
            });

            // thread is finished? take it!
            if ongoing_installation.thread.is_finished() {
                info!("install thread finished");
                // FIXME: avoid expect. we know that it is Some..
                let ongoing_installation = maybe_ongoing_installation
                    .take()
                    .expect("there is an ongoing installation");

                if ongoing_installation.cancel.load(Ordering::Relaxed) {
                    info!("installation was cancelled");
                    add_toast(toasts, ToastKind::Info, "Installation cancelled.");
                    return;
                }

                match ongoing_installation.thread.join() {
                    Ok(Ok(_)) => {
                        info!("installation thread finished with success");
                        add_toast(
                            toasts,
                            ToastKind::Success,
                            "Installation completed successfully!",
                        );
                    }
                    Ok(Err(e)) => {
                        error!("installation thread finished with error:\n{:?}", e);
                        add_toast(
                            toasts,
                            ToastKind::Error,
                            format!("Installation failed:\n{}", e),
                        );
                    }
                    Err(e) => {
                        error!("installation thread panicked:\n{:?}", e);
                        add_toast(
                            toasts,
                            ToastKind::Error,
                            format!("Installation crashed:\n{:?}", e),
                        );
                    }
                };
            }
        } else if !staged_files.is_empty() {
            if ui.button("🔌 install now!").clicked() {
                let game_paths: Vec<_> = staged_files
                    .files
                    .iter()
                    .filter_map(|staged_file| {
                        staged_file.selected.then_some(staged_file.path.clone())
                    })
                    .collect();

                start_usb_install(game_paths, *for_sphaira, maybe_ongoing_installation);
            }
            if ui.button("❌ remove from list").clicked() {
                staged_files.remove_selected();
            }
            // FIXME: fuckjing horrible
            ui.weak(format!(
                "{} selected ({})",
                staged_files
                    .files
                    .iter()
                    .filter(|staged_file| staged_file.selected)
                    .count(),
                humansize::format_size(
                    staged_files
                        .files
                        .iter()
                        .filter(|staged_file| staged_file.selected)
                        .map(|staged_file| staged_file.file_size)
                        .sum::<u64>(),
                    humansize::BINARY
                ),
            ));
        }
    });
}

fn start_usb_install(
    game_paths: Vec<PathBuf>,
    for_sphaira: bool,
    maybe_ongoing_installation: &mut Option<OngoingInstallation>,
) {
    let (progress_len_tx, progress_len_rx) = mpsc::channel::<u64>();
    let (progress_tx, progress_rx) = mpsc::channel::<u64>();

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_thread = cancel.clone();

    *maybe_ongoing_installation = Some(OngoingInstallation {
        progress_len_rx,
        progress_rx,
        thread: std::thread::spawn(move || {
            perform_usb_install(
                &game_paths,
                progress_len_tx,
                progress_tx,
                for_sphaira,
                cancel_thread,
            )
        }),
        last_progress: 0,
        last_progress_len: 1,
        cancel,
    });
}
