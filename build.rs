// Vkontroller - Turns your browser into a virtual game controller
// Copyright (C) 2026  flamfrosticboio
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::env;
use std::process::Command;

const FRONTEND_DIR: &str = "frontend";

fn main() {
    if std::env::var("SKIP_BUILD_SCRIPT").is_ok() {
        return;
    }

    let profile = env::var("PROFILE").unwrap_or_default();

    if profile == "release" {
        println!("cargo:warning=Release build: skipping frontend build (expects dist/ to exist)");
        println!("cargo:rerun-if-changed=frontend/src");
        println!("cargo:rerun-if-changed=frontend/package.json");
        return;
    }

    let status = Command::new("npm")
        .args(["ci", "--omit=dev"])
        .current_dir(FRONTEND_DIR)
        .status()
        .expect("Failed to run 'npm install'");
    assert!(status.success(), "'npm install' failed");

    let status = Command::new("npm")
        .args(["run", "build", "--", "--outDir", "../dist"])
        .current_dir(FRONTEND_DIR)
        .status()
        .expect("Failed to run 'npm run build'");
    assert!(status.success(), "'npm run build' failed");

    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");
}
