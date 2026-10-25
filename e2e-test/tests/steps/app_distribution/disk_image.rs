//! Steps of the disk image: what it holds, its window and its volume name.

use super::{
    checkout, crate_version, ensure, fs, given, off_thread, path_str, run_tool, stage_disk_image,
    text, then, when, Command, Context, E2eWorld, Mounted, Path, Result, APP_NAME,
    BACKGROUND_PICTURE, BUNDLE_DIR, INFO_FILE, WINDOW_LAYOUT,
};

#[given("the disk image the release build made")]
pub(super) fn release_disk_image(world: &mut E2eWorld) -> Result<()> {
    world.distribution.bundles = vec![stage_disk_image()?];
    Ok(())
}

#[when("the user opens it")]
pub(super) async fn open_disk_image(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let image = world.distribution.bundles[0].clone();
        let mount = world.workdir()?.join("volume");
        fs::create_dir_all(&mount)?;
        let output = run_tool(
            "hdiutil",
            &[
                "attach",
                "-nobrowse",
                "-readonly",
                "-mountpoint",
                path_str(&mount)?,
                path_str(&image)?,
            ],
        )?;
        ensure!(
            output.status.success(),
            "the disk image did not open: {}",
            text(&output)
        );
        world.distribution.mount = Some(Mounted(mount));
        Ok(())
    })
    .await
}

pub(super) fn mounted(world: &E2eWorld) -> Result<&Path> {
    Ok(&world
        .distribution
        .mount
        .as_ref()
        .context("the disk image is not open")?
        .0)
}

#[then("it holds the application and a shortcut to the Applications folder")]
pub(super) fn holds_app_and_shortcut(world: &mut E2eWorld) -> Result<()> {
    let volume = mounted(world)?;
    ensure!(
        volume.join(BUNDLE_DIR).join(INFO_FILE).is_file(),
        "the volume holds no application bundle"
    );
    let shortcut = fs::read_link(volume.join("Applications"))
        .context("the volume holds no Applications shortcut")?;
    ensure!(
        shortcut == Path::new("/Applications"),
        "the Applications shortcut points to {}",
        shortcut.display()
    );
    let mut entries: Vec<String> = fs::read_dir(volume)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| !name.starts_with('.'))
        .collect();
    entries.sort();
    ensure!(
        entries == ["Applications", BUNDLE_DIR],
        "the volume shows {entries:?}, expected the application and the Applications shortcut only"
    );
    Ok(())
}

#[then("its window is laid out over the picture that says to drag the application to Applications")]
pub(super) fn window_over_background(world: &mut E2eWorld) -> Result<()> {
    let volume = mounted(world)?;
    let picture = fs::read(volume.join(BACKGROUND_PICTURE))
        .context("the volume holds no background picture")?;
    let packaged = fs::read(checkout().join("app/packaging/dmg-background.png"))?;
    ensure!(
        picture == packaged,
        "the background picture is not the packaged one"
    );
    ensure!(
        volume.join(WINDOW_LAYOUT).is_file(),
        "the volume holds no window layout"
    );
    Ok(())
}

#[then("its volume is named after the application and the version")]
pub(super) fn volume_named(world: &mut E2eWorld) -> Result<()> {
    let volume = mounted(world)?;
    let info = Command::new("diskutil")
        .args(["info", "-plist"])
        .arg(volume)
        .output()?;
    ensure!(info.status.success(), "{}", text(&info));
    let mut extract = Command::new("plutil")
        .args(["-extract", "VolumeName", "raw", "-o", "-", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    std::io::Write::write_all(
        extract.stdin.as_mut().context("plutil's input")?,
        &info.stdout,
    )?;
    let name = extract.wait_with_output()?;
    let name = String::from_utf8_lossy(&name.stdout).trim().to_string();
    let expected = format!("{APP_NAME} {}", crate_version()?);
    ensure!(
        name == expected,
        "the volume is named '{name}', expected '{expected}'"
    );
    Ok(())
}
