use crate::util::*;
use crate::BuildProfile;
use fs_extra::{copy_items, dir::CopyOptions};
use image::imageops::{resize, FilterType};
use regex::Regex;
use std::collections::HashMap;
use std::fs::{copy, create_dir_all, read_to_string, write};
use std::path::Path;
use std::process::Command;
use symlink::symlink_dir;

pub fn build_sdl_for_android(manifest_path: &Path, targets: &Vec<&str>, profile: BuildProfile) {
    let manifest_dir = manifest_path.parent().unwrap();
    let p = Path::new(&*get_env_var("ANDROID_NDK_HOME")).join("ndk-build");

    assert!(Command::new(p)
        .args([
            "NDK_PROJECT_PATH=.",
            "APP_BUILD_SCRIPT=./Android.mk",
            "APP_PLATFORM=android-18"
        ])
        .current_dir(&*get_env_var("SDL"))
        .status()
        .unwrap()
        .success());

    for rust_name in targets {
        let android_name = get_target_android_name(rust_name);
        let rust_dir = manifest_dir
            .join("target")
            .join(rust_name)
            .join(profile.to_string())
            .join("deps");

        create_dir_all(&rust_dir).expect("Unable to create target dir");
        copy(
            Path::new(&*get_env_var("SDL"))
                .join("libs")
                .join(android_name)
                .join("libSDL2.so"),
            rust_dir.join("libSDL2.so"),
        )
        .expect("Unable to copy SDL dependencies");
    }
}

pub fn get_target_android_name(rust_target_name: &str) -> &str {
    match rust_target_name {
        "aarch64-linux-android" => "arm64-v8a",
        "armv7-linux-androideabi" => "armeabi-v7a",
        "i686-linux-android" => "x86",
        _ => {
            panic!("Unknown target: {}", rust_target_name)
        }
    }
}

pub fn get_android_app_id(manifest_path: &Path) -> String {
    get_toml_string(
        manifest_path,
        vec!["package", "metadata", "android", "package_name"],
    )
    .unwrap_or("org.libsdl.app".to_string())
}

fn use_permission(manifest_dir: &Path, permission: &str) {
    change_android_project_file(
        manifest_dir,
        "app/src/main/AndroidManifest.xml",
        vec![("<application android:label=\"@string/app_name\"", format!("<uses-permission android:name=\"android.permission.{permission}\" />\n\t<application android:label=\"@string/app_name\"").as_str())],
    );
}

fn create_android_project(manifest_path: &Path, target_artifacts: &HashMap<String, String>) {
    let manifest_dir = manifest_path.parent().unwrap();
    let appid = get_android_app_id(manifest_path);

    let appname = get_toml_string(
        manifest_path,
        vec!["package", "metadata", "android", "title"],
    )
    .unwrap_or("Untitled".to_string());

    // Copy template project from SDL
    copy_items(
        &[Path::new(&*get_env_var("SDL")).join("android-project")],
        Path::new(manifest_dir).join("target"),
        &CopyOptions::new().skip_exist(false).overwrite(true),
    )
    .unwrap();

    // Create main activity class
    let java_main_folder = manifest_dir
        .join("target/android-project/app/src/main/java")
        .join(str::replace(&appid, ".", "/"));
    create_dir_all(java_main_folder).unwrap();

    change_android_project_file(
        manifest_dir,
        "app/build.gradle",
        vec![("org.libsdl.app", &*appid), ("minSdkVersion 16", "minSdkVersion 26")],
    );

    change_android_project_file(
        manifest_dir,
        "app/src/main/res/values/strings.xml",
        vec![("Game", &*appname)],
    );

    if let Some(val) = get_toml_string(
        manifest_path,
        vec!["package", "metadata", "android", "improve_fullscreen"],
    ) {
        if val == "true" {
            change_android_project_file(
                manifest_dir,
                "app/src/main/java/org/libsdl/app/SDLActivity.java",
                vec![("setContentView(mLayout);", r#"mLayout.setSystemUiVisibility(View.SYSTEM_UI_FLAG_LOW_PROFILE | View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_LAYOUT_STABLE | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN);
                setContentView(mLayout);"#)],
            );
        }
    }

    // Link SDL into project
    if !manifest_dir
        .join("target/android-project/app/jni/SDL")
        .is_dir()
    {
        symlink_dir(
            Path::new(&*get_env_var("SDL")),
            manifest_dir.join("target/android-project/app/jni/SDL"),
        )
        .unwrap();
    }

    {
        let path = manifest_dir.join("target/android-project/app/build.gradle");
        let first = Regex::new(
            r"(externalNativeBuild\s\{)([^\}]*)(abiFilters)([^\}]*)(\})([^\}]*)(\})([^\}]*)(\})",
        )
        .unwrap();
        let second = Regex::new(
            r"(externalNativeBuild\s\{)([^\}]*)(path)([^\}]*)(\})([^\}]*)(\})([^\}]*)(\})",
        )
        .unwrap();
        let string = std::fs::read_to_string(&path).unwrap();
        let mut new = first.replace(
            &string,
            r#"externalNativeBuild {
            cmake {
                arguments "-DANDROID_APP_PLATFORM=android-19", "-DANDROID_STL=c++_static"
                //abiFilters 'armeabi-v7a', 'arm64-v8a', 'x86', 'x86_64'
                abiFilters 'armeabi-v7a', 'arm64-v8a', 'x86'
            }
        }"#,
        );
        let new_string = new.to_string();
        new = second.replace(
            &new_string,
            r#"externalNativeBuild {
            cmake {
                path 'jni/CMakeLists.txt'
            }
        }"#,
        );
        std::fs::write(path, new.to_string()).unwrap();

        change_android_project_file(
            manifest_dir,
            "app/jni/CMakeLists.txt",
            vec![("add_subdirectory(src)", "")],
        )
    }

    // Copy libmain.so to all targets
    for (target, artifact) in target_artifacts {
        let target_android_name = get_target_android_name(target);

        let android_dir = manifest_dir
            .join("target/android-project/app/src/main/jniLibs")
            .join(target_android_name);

        create_dir_all(&android_dir).unwrap();
        copy(artifact, android_dir.join("libmain.so")).unwrap();
    }


    {
        let icon = get_toml_string(
            manifest_path,
            vec!["package", "metadata", "android", "icon"],
        );
        let adaptive_icon_foreground = get_toml_string(
            manifest_path,
            vec!["package", "metadata", "android", "adaptive_icon_foreground"],
        );
        let adaptive_icon_background = get_toml_string(
            manifest_path,
            vec!["package", "metadata", "android", "adaptive_icon_background"],
        );
        let adaptive_icon_monochrome = get_toml_string(
            manifest_path,
            vec!["package", "metadata", "android", "adaptive_icon_monochrome"],
        );
        if (adaptive_icon_foreground.is_some() || adaptive_icon_background.is_some() || adaptive_icon_monochrome.is_some()) && icon.is_some() {
            panic!("You can only specify an icon OR an adaptive icon");
        }
        {
            let res_dir = manifest_dir.join("target/android-project/app/src/main/res");
            let versions = [
                ("mdpi", 48),
                ("hdpi", 72),
                ("xhdpi", 96),
                ("xxhdpi", 144),
                ("xxxhdpi", 192),
            ];
            if let Some(icon) = icon {
                let image = image::open(manifest_dir.join(icon)).unwrap();

                for (res, size) in versions {
                    let dir = res_dir.join(format!("mipmap-{res}"));
                    std::fs::remove_dir_all(&dir).unwrap();
                    std::fs::create_dir_all(&dir).unwrap();
                    let img = resize(&image, size, size, FilterType::Gaussian);
                    img.save(dir.join("ic_launcher.png")).unwrap();
                }
            }

            if adaptive_icon_foreground.is_some() || adaptive_icon_background.is_some() || adaptive_icon_monochrome.is_some() {

                let image_foreground = adaptive_icon_foreground.map(|path| image::open(manifest_dir.join(path)).unwrap());
                let image_background = adaptive_icon_background.map(|path| image::open(manifest_dir.join(path)).unwrap());
                let image_monochrome = adaptive_icon_monochrome.map(|path| image::open(manifest_dir.join(path)).unwrap());

                for (res, size) in versions {
                    let dir = res_dir.join(format!("mipmap-{res}"));
                    std::fs::remove_dir_all(&dir).unwrap();
                    std::fs::create_dir_all(&dir).unwrap();
                    let foreground = image_foreground.as_ref().map(|image| resize(image, size, size, FilterType::Gaussian));
                    let background = image_background.as_ref().map(|image| resize(image, size, size, FilterType::Gaussian));
                    let monochrome = image_monochrome.as_ref().map(|image| resize(image, size, size, FilterType::Gaussian));

                    if let Some(image) = foreground {
                        image.save(dir.join("ic_launcher_foreground.png")).unwrap();
                    }
                    if let Some(image) = background {
                        image.save(dir.join("ic_launcher_background.png")).unwrap();
                    }
                    if let Some(image) = monochrome {
                        image.save(dir.join("ic_launcher_monochrome.png")).unwrap();
                    }

                    let mut xml = String::from(r#"<?xml version="1.0" encoding="utf-8"?><adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">"#);
                    if image_foreground.is_some() {
                        xml += r#"<foreground android:drawable="@mipmap/ic_launcher_foreground" />"#;
                    }
                    if image_background.is_some() {
                        xml += r#"<background android:drawable="@mipmap/ic_launcher_background" />"#;
                    }
                    if image_monochrome.is_some() {
                        xml += r#"<monochrome android:drawable="@mipmap/ic_launcher_monochrome" />"#;
                    }
                    xml += r#"</adaptive-icon>"#;
                    std::fs::write(dir.join("ic_launcher.xml"), xml).unwrap();
                }
            }
        }
    }

    if let Some(permissions) = get_toml_string_array(manifest_path, vec!["package", "metadata", "android", "permissions"]) {
        for permission in permissions {
            use_permission(manifest_dir, &permission);
        }
    }

    let apk_output_dir = manifest_dir.join("target/android-project/app/build/outputs/apk");
    if apk_output_dir.exists() {
        std::fs::remove_dir_all(apk_output_dir).unwrap();
    }
}

fn change_android_project_file(
    manifest_dir: &Path,
    file_name: &str,
    replacements: Vec<(&str, &str)>,
) {
    let mut content = read_to_string(
        Path::new(&*get_env_var("SDL"))
            .join("android-project")
            .join(file_name),
    )
    .expect("Unable to read manifest file");

    for (from, to) in replacements {
        content = content.replace(from, to);
    }

    write(
        manifest_dir.join("target/android-project").join(file_name),
        &content,
    )
    .expect("Unable to write file");
}

pub fn sign_android(manifest_path: &Path, ks_file: Option<String>, ks_pass: Option<String>) {
    let manifest_dir = manifest_path.parent().unwrap();
    let release_dir = manifest_dir.join("target/android-project/app/build/outputs/apk/release");

    // Find android build tools.
    let tool_paths =
        std::fs::read_dir(Path::new(&*get_env_var("ANDROID_HOME")).join("build-tools")).unwrap();
    let mut tool_paths: Vec<String> = tool_paths
        .map(|d| {
            d.unwrap()
                .path()
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap()
        })
        .collect();
    tool_paths.sort();
    tool_paths.reverse();
    let tools_version = tool_paths[0].clone();
    println!("Using build-tools: {}", tools_version);

    // Determine key file. Generate if needed.
    let (key_file, key_pass) = if let Some(ks_file) = ks_file {
        (ks_file, ks_pass.expect("Need keystore password"))
    } else {
        let key_path = release_dir.join("app-release.jks");
        if !key_path.exists() {
            println!("Generating keyfile...");
            assert!(Command::new("keytool")
                .arg("-genkey")
                .arg("-dname")
                .arg("CN=Unknown, OU=Unknown, O=Unknown, L=Unknown, S=Unknown, C=Unknown")
                .arg("-storepass")
                .arg("android")
                .arg("-keystore")
                .arg(key_path.clone())
                .arg("-keyalg")
                .arg("RSA")
                .arg("-keysize")
                .arg("2048")
                .arg("-validity")
                .arg("10000")
                .status()
                .unwrap()
                .success());
        }

        (
            key_path.into_os_string().into_string().unwrap(),
            "pass:android".to_string(),
        )
    };

    println!("Using keyfile: {}", key_file);

    // Run zipalign.
    let zipalign_path = Path::new(&*get_env_var("ANDROID_HOME"))
        .join("build-tools")
        .join(tools_version.clone())
        .join("zipalign");

    assert!(Command::new(zipalign_path)
        .arg("-v")
        .arg("-f")
        .arg("-p")
        .arg("4")
        .arg(release_dir.join("app-release-unsigned.apk"))
        .arg(release_dir.join("app-release-unsigned-aligned.apk"))
        .status()
        .unwrap()
        .success());

    // Run apksigner
    let apksigner_path = Path::new(&*get_env_var("ANDROID_HOME"))
        .join("build-tools")
        .join(tools_version)
        .join("apksigner");

    assert!(Command::new(apksigner_path)
        .arg("sign")
        .arg("-ks")
        .arg(key_file)
        .arg("-ks-pass")
        .arg(key_pass)
        .arg("-out")
        .arg(release_dir.join("app-release.apk"))
        .arg(release_dir.join("app-release-unsigned-aligned.apk"))
        .status()
        .unwrap()
        .success());
}

// keytool -android blabla -genkey -v -keystore my-release-key.jks -keyalg RSA -keysize 2048 -validity 10000 -alias my-alias
// /home/micke/Android/Sdk/build-tools/30.0.3/zipalign -v -p 4 app-release-unsigned.apk app-release-unsigned-aligned.apk
// /home/micke/Android/Sdk/build-tools/30.0.3/apksigner sign -ks my-release-key.jks -ks-pass pass:android -out app-release.apk app-release-unsigned-aligned.apk

pub fn build_android_project(
    manifest_path: &Path,
    target_artifacts: &HashMap<String, String>,
    profile: BuildProfile,
    ks_file: Option<String>,
    ks_pass: Option<String>,
) {
    let manifest_dir = manifest_path.parent().unwrap();

    create_android_project(manifest_path, target_artifacts);

    let gradle_task = match profile {
        BuildProfile::Debug => "assembleDebug",
        BuildProfile::Release => "assembleRelease",
    };

    assert!(Command::new("./gradlew")
        .args([gradle_task])
        .current_dir(manifest_dir.join("./target/android-project"))
        .status()
        .unwrap()
        .success());

    if matches!(profile, BuildProfile::Release) {
        sign_android(manifest_path, ks_file, ks_pass);
    }
}
