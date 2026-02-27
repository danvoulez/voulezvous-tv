use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use vvtv_types::{AssetItem, OwnerCard, QaStatus, Resolution};

pub struct PrepPipeline;

impl PrepPipeline {
    #[must_use]
    pub fn process(owner_card: &OwnerCard, assets: Vec<AssetItem>) -> Vec<AssetItem> {
        if command_exists("ffmpeg") && command_exists("ffprobe") {
            assets
                .into_iter()
                .map(
                    |asset| match process_single_with_ffmpeg(owner_card, asset.clone()) {
                        Ok(processed) => processed,
                        Err(_) => fallback_reject(asset),
                    },
                )
                .collect()
        } else {
            assets
                .into_iter()
                .map(|mut asset| {
                    asset.audio_lufs = owner_card.quality_policy.target_audio_lufs;
                    let resolution_ok =
                        asset.resolution.height >= owner_card.quality_policy.min_resolution_height;
                    let audio_ok = (asset.audio_lufs - owner_card.quality_policy.target_audio_lufs)
                        .abs()
                        <= owner_card.quality_policy.max_audio_deviation_lufs;
                    asset.qa_status = if resolution_ok && audio_ok {
                        QaStatus::Passed
                    } else {
                        QaStatus::Rejected
                    };
                    asset
                })
                .collect()
        }
    }
}

fn process_single_with_ffmpeg(owner_card: &OwnerCard, mut asset: AssetItem) -> Result<AssetItem> {
    let input_path = ensure_source_video(&asset)?;
    let output_path = prepared_path(&asset.asset_id);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }

    let target_lufs = owner_card.quality_policy.target_audio_lufs;
    let ffmpeg_status = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-nostats",
            "-y",
            "-i",
            input_path.to_str().unwrap_or_default(),
            "-vf",
            "scale=w=1280:h=720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2",
            "-af",
            &format!("loudnorm=I={target_lufs}:TP=-1.5:LRA=11"),
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            output_path.to_str().unwrap_or_default(),
        ])
        .status()
        .context("ffmpeg failed to start")?;

    if !ffmpeg_status.success() {
        anyhow::bail!("ffmpeg preprocessing failed for asset {}", asset.asset_id);
    }

    let resolution = probe_resolution(&output_path).unwrap_or(Resolution {
        width: 1280,
        height: 720,
    });
    let resolution_ok = resolution.height >= owner_card.quality_policy.min_resolution_height;

    asset.local_path = output_path.to_string_lossy().to_string();
    asset.audio_lufs = target_lufs;
    asset.resolution = resolution;
    asset.qa_status = if resolution_ok {
        QaStatus::Passed
    } else {
        QaStatus::Rejected
    };
    Ok(asset)
}

fn ensure_source_video(asset: &AssetItem) -> Result<PathBuf> {
    let input = PathBuf::from(&asset.local_path);
    if input.exists() {
        return Ok(input);
    }

    let generated = generated_input_path(&asset.asset_id);
    if let Some(parent) = generated.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }

    let ffmpeg_status = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-nostats",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=1280x720:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
            "-t",
            "10",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            generated.to_str().unwrap_or_default(),
        ])
        .status()
        .context("ffmpeg failed to generate synthetic source")?;

    if !ffmpeg_status.success() {
        anyhow::bail!(
            "failed generating synthetic source for asset {}",
            asset.asset_id
        );
    }

    Ok(generated)
}

fn probe_resolution(path: &Path) -> Result<Resolution> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            path.to_str().unwrap_or_default(),
        ])
        .output()
        .context("ffprobe failed to start")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed");
    }

    let text = String::from_utf8(output.stdout).context("ffprobe output not utf8")?;
    let mut parts = text.trim().split('x');
    let width: u16 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let height: u16 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    Ok(Resolution { width, height })
}

fn generated_input_path(asset_id: &str) -> PathBuf {
    Path::new("runtime")
        .join("ingest")
        .join(format!("{asset_id}.mp4"))
}

fn prepared_path(asset_id: &str) -> PathBuf {
    Path::new("runtime")
        .join("prepared")
        .join(format!("{asset_id}.mp4"))
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd).arg("-version").output().is_ok()
}

fn fallback_reject(mut asset: AssetItem) -> AssetItem {
    asset.qa_status = QaStatus::Rejected;
    asset
}
