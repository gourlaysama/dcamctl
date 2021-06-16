use std::io::{Stdout, Write};

use crate::cam_info::CamInfo;
use anyhow::*;
use futures::{FutureExt, Stream, StreamExt};
use log::*;
use termion::{event::Key, input::TermRead};
use tokio::{signal::unix::SignalKind, sync::oneshot::Sender};

enum Command {
    Quit,
    ZoomIn,
    ZoomOut,
    Nothing,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    QualityUp,
    QualityDown,
}

#[derive(Debug)]
struct CamControl {
    quit: Sender<()>,
    port: u16,
    cam_info: CamInfo,
    stdout: Stdout,
}

impl CamControl {
    async fn new(quit: Sender<()>, port: u16) -> Result<CamControl> {
        let cam_info = get_cam_info(port, true).await?;

        Ok(CamControl {
            quit,
            port,
            cam_info,
            stdout: std::io::stdout(),
        })
    }

    async fn refresh(&mut self) -> Result<()> {
        let new = get_cam_info(self.port, false).await?;

        self.cam_info.curvals = new.curvals;

        Ok(())
    }

    fn display_status(&mut self) -> Result<()> {
        if let Some((zoom_idx, zoom_end)) = self.zoom_index() {
            let p = (100 * zoom_idx) / zoom_end;
            let q = self.cam_info.curvals.quality;
            if log_enabled!(log::Level::Error) {
                write!(self.stdout, "Zoom: {:2} %, Quality: {:2} %\r", p, q)?;
                self.stdout.flush()?;
            }
        }

        Ok(())
    }

    fn zoom_index(&self) -> Option<(usize, usize)> {
        let h = &self.cam_info.avail.as_ref()?.zoom;
        let idx = h.iter().position(|e| {
            if let Ok(i) = e.parse::<u16>() {
                i == self.cam_info.curvals.zoom
            } else {
                false
            }
        })?;

        Some((idx, h.len()))
    }

    fn increment_zoom_index(&self) -> Result<usize> {
        let (idx, len) = self
            .zoom_index()
            .ok_or_else(|| anyhow!("internal error while zooming in"))?;

        if idx < len {
            Ok(idx + 1)
        } else {
            Ok(idx)
        }
    }

    fn decrement_zoom_index(&self) -> Result<usize> {
        let (idx, _) = self
            .zoom_index()
            .ok_or_else(|| anyhow!("internal error while zooming out"))?;

        if idx > 0 {
            Ok(idx - 1)
        } else {
            Ok(0)
        }
    }
}

async fn get_cam_info(port: u16, init: bool) -> Result<CamInfo> {
    let show = if init { "1" } else { "0" };
    let c = reqwest::get(format!(
        "http://127.0.0.1:{}/status.json?show_avail={}",
        port, show
    ))
    .await?
    .json::<CamInfo>()
    .await?;

    trace!("{:?}", c);

    Ok(c)
}

pub async fn process_commands(port: u16) -> Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let j = tokio::task::spawn(async move {
        match CamControl::new(tx, port).await {
            Ok(c) => {
                match process_commands_inner(c).await {
                    Ok(_) => {}
                    Err(e) => error!("{}", e),
                };
            }
            Err(e) => error!("{}", e),
        };
    });

    futures::future::select(rx, j).await;

    Ok(())
}

async fn process_commands_inner(control: CamControl) -> Result<()> {
    let mut cmds = input_commands().boxed();
    let mut control = control;

    control.display_status()?;
    while let Some(cmd) = cmds.next().await {
        match cmd {
            Command::Quit => {
                if log_enabled!(log::Level::Error) {
                    write!(control.stdout, "{}", termion::clear::CurrentLine)?;
                    control.stdout.flush()?;
                }
                control
                    .quit
                    .send(())
                    .map_err(|_| anyhow!("broken channel"))?;
                break;
            }
            Command::Nothing => {}
            Command::ZoomIn => {
                let new_zoom = &control.increment_zoom_index()?;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/ptz?zoom={}",
                    control.port, new_zoom
                ))
                .await?;
            }
            Command::ZoomOut => {
                let new_zoom = &control.decrement_zoom_index()?;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/ptz?zoom={}",
                    control.port, new_zoom
                ))
                .await?;
            }
            Command::PanLeft => {
                let new_x = &control.cam_info.curvals.crop_x.max(1) - 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_x?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::PanRight => {
                let new_x = &control.cam_info.curvals.crop_x + 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_x?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::PanUp => {
                let new_x = &control.cam_info.curvals.crop_y.max(1) - 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_y?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::PanDown => {
                let new_x = &control.cam_info.curvals.crop_y + 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_y?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::QualityUp => {
                let new_q = &control.cam_info.curvals.quality + 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/quality?set={}",
                    control.port, new_q
                ))
                .await?;
            },
            Command::QualityDown => {
                let new_q = &control.cam_info.curvals.quality - 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/quality?set={}",
                    control.port, new_q
                ))
                .await?;
            },
        }

        control.refresh().await?;
        control.display_status()?;
    }

    Ok(())
}

pub async fn stop_signals() -> Result<()> {
    let mut i = tokio::signal::unix::signal(SignalKind::interrupt())?;
    let mut t = tokio::signal::unix::signal(SignalKind::terminate())?;
    let mut q = tokio::signal::unix::signal(SignalKind::quit())?;

    let is = i.recv().boxed_local();
    let ts = t.recv().boxed_local();
    let qs = q.recv().boxed_local();

    futures::future::select_all(vec![is, ts, qs]).await;

    Ok(())
}

fn input_commands() -> impl Stream<Item = Command> {
    let keys = stdin_stream();

    use Command::*;
    keys.map(|k| match k {
        Key::Char('q') => Quit,
        Key::Char('z') => ZoomIn,
        Key::Char('Z') => ZoomOut,
        Key::Char('t') => QualityUp,
        Key::Char('T') => QualityDown,
        Key::Left => PanLeft,
        Key::Right => PanRight,
        Key::Up => PanUp,
        Key::Down => PanDown,
        _ => Nothing,
    })
}

fn stdin_stream() -> impl Stream<Item = Key> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Key>(16);

    tokio::task::spawn_blocking(move || {
        let mut keys = std::io::stdin().keys();
        while let Some(Ok(k)) = keys.next() {
            tx.blocking_send(k).unwrap();
        }
    });

    let stream = async_stream::stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
    };

    stream
}
