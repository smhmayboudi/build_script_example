#![allow(clippy::too_many_lines)]

use core::panic;
use git2::{IntoCString, Repository};
use std::{collections::HashMap, env, ffi::OsStr, fs, io, path::Path, process::Command};

const MACOSX_DEPLOYMENT_TARGET: &str = "12.0"; // empty string means get the latest version from system

enum BuildType {
    Cmake = 0,
    Make = 1,
    Shell = 2,
}
enum LibraryType {
    Static,
    Dynamic,
}

struct Dependency {
    pub name: String,
    pub library_type: LibraryType,
}

struct BuildConfig {
    pub name: String,
    pub git_url: String,
    pub build_type: BuildType,
    pub library_type: LibraryType,
    pub make_file_path_prefix: String,
    pub dependencies: Vec<Dependency>,
    pub relative_path_to_cxx_file: String,
    pub cmake_configurations: Vec<String>,
    pub relative_path_to_cmake_file: String,
    pub relative_path_to_rust_related_cxx_file: String,
}

struct Builder {
    sources: Vec<BuildConfig>,
}
impl Builder {
    // Bare empty initilization
    pub fn new(elements_length: usize) -> Builder {
        Builder {
            sources: Vec::with_capacity(elements_length),
        }
    }

    pub fn push(&mut self, config: BuildConfig) {
        self.sources.push(config);
    }

    pub fn initilize_sources(&self) {
        // Lz4
        self.push(BuildConfig {
            name: String::from("lz4"),
            build_type: BuildType::Cmake,
            dependencies: Vec::new(),
            library_type: LibraryType::Static,
            make_file_path_prefix: String::new(),
            git_url: String::from("https://github.com/lz4/lz4.git"),
            relative_path_to_cmake_file: String::from("build/cmake/"),
            relative_path_to_cxx_file: String::from("src/compression/cxx/lz4/"),
            relative_path_to_rust_related_cxx_file: String::from("src/compression/lz4.rs"),
            cmake_configurations: vec![
                String::from("-DBUILD_SHARED_LIBS=FALSE"),
                String::from("-DBUILD_STATIC_LIBS=TRUE"),
            ],
        });

        // Lzma
        self.push(BuildConfig {
            name: String::from("lzma"),
            build_type: BuildType::Cmake,
            dependencies: Vec::new(),
            cmake_configurations: Vec::new(),
            library_type: LibraryType::Static,
            make_file_path_prefix: String::new(),
            relative_path_to_cmake_file: String::new(),
            git_url: String::from("https://github.com/WolfEngine/lzma.git"),
            relative_path_to_cxx_file: String::from("src/compression/cxx/lzma/"),
            relative_path_to_rust_related_cxx_file: String::from("src/compression/lzma.rs"),
        });

        // Platform-specific value declareation
        let lua_lib_name: &str;
        let build_type: BuildType;
        let dependencies: Vec<Dependency>;
        if cfg!(windows) {
            lua_lib_name = "luajit";
            build_type = BuildType::Shell;

            dependencies = vec![Dependency {
                name: String::from("lua51"),
                library_type: LibraryType::Static,
            }];
        } else {
            dependencies = Vec::new();
            lua_lib_name = "luajit-5.1";
            build_type = BuildType::Make;
        };

        // LuaJIT
        self.push(BuildConfig {
            build_type,
            dependencies,
            name: String::from(lua_lib_name),
            cmake_configurations: Vec::new(),
            library_type: LibraryType::Static,
            relative_path_to_cmake_file: String::new(),
            make_file_path_prefix: String::from("/usr/local/"),
            git_url: String::from("https://github.com/LuaJIT/LuaJIT.git"),
            relative_path_to_cxx_file: String::from("src/script/cxx/luaJIT/"),
            relative_path_to_rust_related_cxx_file: String::from("src/script/lua.rs"),
        });
    }
}

fn start() {
    //first build proto buffers
    let paths = fs::read_dir("./proto").expect("could not read from proto folder");

    for path in paths {
        let p = path.unwrap().path();
        if let Some(extension) = p.extension() {
            if extension == "proto" {
                tonic_build::compile_protos(p).unwrap();
            }
        }
    }

    //get the current path
    let current_dir_path = env::current_dir().expect("could not get current directory");

    let current_dir = current_dir_path
        .to_str()
        .expect("could not get a &str from current directory");

    //get current build profile
    let profile_str = env::var("PROFILE").expect("Build failed: could not get PROFILE");

    //get opt_level
    let opt_level_str =
        env::var("OPT_LEVEL").expect("Build failed: could not get OPT_LEVEL profile");

    //set cmake build config
    let build_profile = match &opt_level_str[..] {
        "0" => "Debug",
        "1" | "2" | "3" => {
            if profile_str == "debug" {
                "RelWithDebInfo"
            } else {
                "Release"
            }
        }
        "s" | "z" => "MinSizeRel",
        _ => {
            if profile_str == "debug" {
                "Debug"
            } else {
                "Release"
            }
        }
    };

    //get current target os
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Build failed: could not get target OS");

    // make sure set the necessery enviroment variables for OSX
    if target_os == "macos" {
        if MACOSX_DEPLOYMENT_TARGET.is_empty() {
            let file = fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist")
                .expect("could not read SystemVersion.plist");

            let cur = io::Cursor::new(file.as_bytes());

            let v = plist::Value::from_reader(cur).expect("could not read value from plist");

            let version = v
                .as_dictionary()
                .and_then(|d| d.get("ProductVersion")?.as_string())
                .expect("SystemVersion.plist is not a dictionary");

            env::set_var("MACOSX_DEPLOYMENT_TARGET", version);
        } else {
            env::set_var("MACOSX_DEPLOYMENT_TARGET", MACOSX_DEPLOYMENT_TARGET);
        }
    }

    // create vectors for storing rust & cxx sources
    let mut cxx_srcs: Vec<String> = Vec::new();
    let mut rust_srcs: Vec<String> = Vec::new();
    let mut include_srcs: Vec<String> = Vec::new();
    let mut lib_deps = Vec::<Dependency>::new();
    let mut lib_paths: HashMap<String, (String, bool)> = HashMap::new();

    // create deps folder
    make_folder(&format!("{}/deps/", current_dir));

    // iterate over git repositories
    for (k, source) in sources.iterator.iter_mut().enumerate() {
        // store deps libraries
        lib_deps.append(&mut source.make_file_path_prefix);

        // check git project already exists
        let git_repo_path = format!("{}/deps/{}", current_dir, k);

        let prefix_path = source.relative_path_to_rust_related_cxx_file;

        //path to the Cmake folder
        let path_to_cmake_folder = source.git_url;

        //git project just opened
        let path_to_lib = format!(
            "{}/{}build/{}/{}/lib/",
            git_repo_path, path_to_cmake_folder, build_profile, prefix_path
        );

        // let build_dir = format!("{}/deps/{}/build/{}", current_dir, k, build_profile);
        // panic!("{}", build_dir);
        // println!(
        //     "cargo:rustc-link-search=native={}{}/lib",
        //     build_dir, prefix_path
        // );

        let build = match Repository::open(git_repo_path.clone()) {
            Ok(_g) => !Path::new(&path_to_lib).exists(),
            Err(_e) =>
            // try cloning it again
            {
                let url = source.git_url.clone();

                if !url.is_empty() {
                    let cloned = Repository::clone_recurse(url, git_repo_path.clone());

                    if cloned.is_err() {
                        panic!("could not clone '{}' because: {:?}", url, cloned.err());
                    }
                }

                true
            }
        };

        if build {
            match v.0 {
                BuildType::Cmake => {
                    build_cmake(build_profile, &git_repo_path, &source);
                }
                BuildType::Make => {
                    build_make(build_profile, &opt_level_str, &git_repo_path, &source);
                }
                BuildType::Shell => {
                    // build with custom shell script should be implement
                    // Use unimplemented! macro from the std library if it's critical
                }
            }
        }

        //link library static or dynamic
        let link_static = source.cmake_configurations;

        //cxx src path
        let cxx_src_path = v.5;

        //store it for later
        cxx_srcs.push(cxx_src_path.to_string());

        //rust src path
        let rust_src_path = v.6;

        //store it for later
        rust_srcs.push(rust_src_path.to_string());

        let prefix_path = v.7;

        //path to the includes
        let path_to_include = format!(
            "{}/{}build/{}/{}/include/",
            git_repo_path, path_to_cmake_folder, build_profile, prefix_path
        );

        include_srcs.push(path_to_include.to_string());

        //path to the libraries
        let _r = lib_paths.insert(k.to_string(), (path_to_lib.to_string(), link_static));
    }

    // build cxx lib
    let mut build_cxx = cxx_build::bridges(rust_srcs);

    //build_cxx.flag_if_supported("/std:c++latest");
    let _r = build_cxx.flag_if_supported("-std=c++17");
    let _r = build_cxx.flag_if_supported("-std=c17");
    let _r = build_cxx.flag_if_supported("-Wall");
    let _r = build_cxx.flag_if_supported("-fPIC");

    //add DEBUG or NDEBUG flags
    if profile_str == "debug" {
        let _r = build_cxx.define("DEBUG", "DEBUG");
    } else {
        let _r = build_cxx.define("NDEBUG", "NDEBUG");
    }

    //add necessery includes/libs or defines based on target OS
    match target_os.as_ref() {
        "macos" | "linux" => {
            let _r = build_cxx.include("/usr/local/include/");
        }
        "windows" => {
            let _r = build_cxx.define("WIN32", "WIN32");
            let _r = build_cxx.define("_WINDOWS", "_WINDOWS");

            if profile_str == "debug" {
                println!("cargo:rustc-link-lib=msvcrtd");
            } else {
                println!("cargo:rustc-link-lib=msvcrt");
            }
        }
        _ => panic!("Build failed: target os \"{}\" undefined", target_os),
    }

    // include all cxx files
    for iter in cxx_srcs {
        //include all c/cpp files
        let _r = fs::read_dir(&iter).map(|paths| {
            for iter in paths {
                if let Ok(path) = iter {
                    let p = path.path();

                    let ext = p
                        .extension()
                        .unwrap_or_else(|| OsStr::new(""))
                        .to_ascii_lowercase();

                    if p.is_file() {
                        let file_path_os_string = p.into_os_string();
                        let file_path_str = file_path_os_string.to_str().unwrap();

                        println!("cargo:rerun-if-changed={}", file_path_str);

                        if ext == "c" || ext == "cc" || ext == "cpp" || ext == "cxx" || ext == "c++"
                        {
                            let _r = build_cxx.file(Path::new(file_path_str));
                        }
                    }
                }
            }
        });
    }

    //include all includes from CMakes
    for iter in include_srcs {
        println!("cargo:rerun-if-changed={}", iter);

        let _r = build_cxx.include(Path::new(&iter));
    }

    //link dependencies
    for (lib, static_) in lib_deps {
        if static_ {
            println!("cargo:rustc-link-lib=static={}", lib);
        } else {
            println!("cargo:rustc-link-lib=dylib={}", lib);
        }
    }

    //link libraries
    for iter in lib_paths {
        println!("cargo:rerun-if-changed={}", iter.1 .0);
        println!("cargo:rustc-link-search=native={}", iter.1 .0);

        if iter.1 .1 {
            println!("cargo:rustc-link-lib=static={}", iter.0);
        } else {
            println!("cargo:rustc-link-lib=dylib={}", iter.0);
        }
    }

    //rename cxx library to prevent con
    build_cxx.compile("wolf_system_cxx");
}

fn build_cmake(p_cmake_build_type: &str, p_git_repo_path: &str, p_value: &BuildConfig) {
    //get path to CMakeLists file
    let cmake_parent_path = format!("{}/{}", p_git_repo_path, p_value.2);

    //get cmake configs
    let cmake_config_args = &p_value.3;

    //check build folder
    let build_folder_str = format!("{}/build", cmake_parent_path);
    let build_folder_path = Path::new(&build_folder_str);

    if !build_folder_path.exists() {
        fs::create_dir(build_folder_path).unwrap_or_else(|_| {
            panic!(
                "could not create build folder for cmake {}CMakeLists.txt",
                cmake_parent_path
            )
        });
    }

    //set installfolder
    let install_folder_str = format!("{}/{}", build_folder_str, p_cmake_build_type);

    //configure cmake
    let mut out = Command::new("cmake")
        .current_dir(&cmake_parent_path)
        .args(["-Wdev", "--debug-output", "-B", "build"])
        .arg(".")
        .args(cmake_config_args)
        .arg(format!("-DCMAKE_INSTALL_PREFIX:PATH={}", install_folder_str).as_str())
        .arg(format!("-DCMAKE_BUILD_TYPE={}", p_cmake_build_type).as_str())
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "Build failed: could not configure cmake for {}CMakeLists.txt",
                cmake_parent_path
            )
        });

    if !out.status.success() {
        panic!(
            "Build failed: CMake project was not configured because: {:?}",
            out.stderr.into_c_string()
        );
    }

    // build cmake
    out = Command::new("cmake")
        .current_dir(&cmake_parent_path)
        .args([
            "--build",
            "build",
            "--target",
            "install",
            "--config",
            p_cmake_build_type,
            "--parallel",
            "8",
        ])
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "Build failed: could not run cmake for {}CMakeLists.txt",
                cmake_parent_path
            )
        });

    if !out.status.success() {
        panic!(
            "CMake Build failed for {} because: {:?}",
            cmake_parent_path,
            out.stderr.into_c_string()
        );
    }
}

fn build_make(
    p_make_build_profile: &str,
    _p_make_opt_level: &str,
    p_git_repo_path: &str,
    p_value: &BuildConfig,
) {
    //get path to make file
    let make_parent_path = format!("{}/{}", p_git_repo_path, p_value.2);

    //check build folder
    let build_folder_str = format!("{}/build", make_parent_path);

    //check install folder
    let install_folder_str = format!("{}/{}", build_folder_str, p_make_build_profile);

    make_folder(&make_parent_path);
    make_folder(&build_folder_str);
    make_folder(&install_folder_str);

    // let c_flag: String;
    // let cxx_flag: String;
    // if p_make_build_profile == "Debug" || p_make_build_profile == "RelWithDebInfo" {
    //     c_flag = format!("CFLAGS='-g -O{} -w'", p_make_opt_level);
    //     cxx_flag = format!("CXXFLAGS='-g -O{} -w'", p_make_opt_level);
    // } else {
    //     c_flag = format!("CFLAGS='-O{} -w'", p_make_opt_level);
    //     cxx_flag = format!("CXXFLAGS='-O{} -w'", p_make_opt_level);
    // }

    let install_flag = format!("DESTDIR={}", &install_folder_str);

    //configure cmake
    let out = std::process::Command::new("make")
        .current_dir(&make_parent_path)
        .args(["install", &install_flag])
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "Build failed: could not build make for {}MakeFile",
                make_parent_path
            )
        });

    if !out.status.success() {
        panic!(
            "Build failed: Make project was not build because: {:?}",
            out.stderr.into_c_string()
        );
    }
}

fn make_folder(path: &str) {
    //check folder exists
    let p = Path::new(&path);

    if !p.exists() {
        fs::create_dir(p).unwrap_or_else(|_| panic!("could not create folder {}", path));
    }
}

fn main() {}
