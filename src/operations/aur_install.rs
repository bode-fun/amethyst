use std::env::set_current_dir;
use std::fs::remove_dir_all;
use std::path::Path;
use std::process::Command;
use std::{env, fs};

use crate::internal::commands::ShellCommand;
use crate::internal::error::SilentUnwrap;
use crate::internal::exit_code::AppExitCode;
use crate::internal::rpc::rpcinfo;
use crate::internal::{crash, prompt};
use crate::{info, log, Options};

pub fn aur_install(a: Vec<String>, options: Options) {
    let url = crate::internal::rpc::URL;
    let cachedir = format!("{}/.cache/ame/", env::var("HOME").unwrap());
    let verbosity = options.verbosity;
    let noconfirm = options.noconfirm;

    if verbosity >= 1 {
        log(format!("Installing from AUR: {:?}", &a));
    }

    info(format!("Installing packages {} from the AUR", a.join(", ")));

    for package in a {
        let rpcres = rpcinfo(package);

        if !rpcres.found {
            break;
        }

        let pkg = &rpcres.package.as_ref().unwrap().name;

        if verbosity >= 1 {
            log(format!("Cloning {} into cachedir", pkg));
        }

        info("Cloning package source".to_string());

        set_current_dir(Path::new(&cachedir)).unwrap();
        ShellCommand::git()
            .arg("clone")
            .arg(format!("{}/{}", url, pkg))
            .wait()
            .silent_unwrap(AppExitCode::GitError);

        if verbosity >= 1 {
            log(format!(
                "Cloned {} into cachedir, moving on to resolving dependencies",
                pkg
            ));
            log(format!(
                "Raw dependencies for package {} are:\n{:?}",
                pkg,
                rpcres.package.as_ref().unwrap().depends.join(", ")
            ));
            log(format!(
                "Raw makedepends for package {} are:\n{:?}",
                pkg,
                rpcres.package.as_ref().unwrap().make_depends.join(", ")
            ));
        }

        // dep sorting
        info("Sorting dependencies".to_string());
        let sorted = crate::internal::sort(&rpcres.package.as_ref().unwrap().depends, options);
        info("Sorting make dependencies".to_string());
        let md_sorted =
            crate::internal::sort(&rpcres.package.as_ref().unwrap().make_depends, options);

        if verbosity >= 1 {
            log(format!(
                "Sorted dependencies for {} are:\n{:?}",
                pkg, &sorted
            ));
            log(format!(
                "Sorted makedepends for {} are:\n{:?}",
                pkg, &md_sorted
            ));
        }

        let newopts = Options {
            verbosity,
            noconfirm,
            asdeps: true,
        };

        if !sorted.nf.is_empty() || !md_sorted.nf.is_empty() {
            crash(
                format!(
                    "Could not find dependencies {} for package {}, aborting",
                    sorted.nf.join(", "),
                    pkg
                ),
                AppExitCode::MissingDeps,
            );
        }

        if !noconfirm {
            let p1 = prompt(
                format!(
                    "Would you like to review {}'s PKGBUILD (and any .install files if present)?",
                    pkg
                ),
                false,
            );
            let editor: &str = &env::var("PAGER").unwrap_or_else(|_| "less".parse().unwrap());

            if p1 {
                Command::new(editor)
                    .arg(format!("{}/PKGBUILD", pkg))
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();

                let status = ShellCommand::bash()
                    .arg("-c")
                    .arg(format!("ls {}/*.install &> /dev/null", pkg))
                    .wait()
                    .silent_unwrap(AppExitCode::Other);

                if status.success() {
                    ShellCommand::bash()
                        .arg("-c")
                        .arg(format!("{} {}/*.install", editor, pkg))
                        .wait()
                        .silent_unwrap(AppExitCode::Other);
                }

                let p2 = prompt(format!("Would you still like to install {}?", pkg), true);
                if !p2 {
                    fs::remove_dir_all(format!("{}/{}", cachedir, pkg)).unwrap();
                    crash("Not proceeding".to_string(), AppExitCode::UserCancellation);
                }
            }
        }

        // dep installing
        info("Moving on to install dependencies".to_string());
        if !sorted.repo.is_empty() {
            crate::operations::install(sorted.repo, newopts);
            crate::operations::install(md_sorted.repo, newopts);
        }
        if !sorted.aur.is_empty() {
            crate::operations::aur_install(sorted.aur, newopts);
            crate::operations::aur_install(md_sorted.aur, newopts);
        }

        let mut makepkg_args = vec!["-rsic", "--skippgp"];
        if options.asdeps {
            makepkg_args.push("--asdeps")
        }
        if options.noconfirm {
            makepkg_args.push("--noconfirm")
        }

        // package building and installing
        info("Building time!".to_string());
        set_current_dir(format!("{}/{}", cachedir, pkg)).unwrap();
        let status = ShellCommand::makepkg()
            .args(makepkg_args)
            .wait()
            .silent_unwrap(AppExitCode::MakePkgError);

        if !status.success() {
            fs::remove_dir_all(format!("{}/{}", cachedir, pkg)).unwrap();
            crash(
                format!("Error encountered while installing {}, aborting", pkg),
                AppExitCode::PacmanError,
            );
        }

        set_current_dir(&cachedir).unwrap();
        remove_dir_all(format!("{}/{}", cachedir, &pkg)).unwrap();

        // pushes package to database
        crate::database::add(rpcres.package.unwrap(), options);
    }
}
