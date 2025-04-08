/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::fs::{self, File};

use cucumber::cli;

#[derive(cli::Args)]
pub(crate) struct CustomTckOpts {
    /// The folder to write the JUnit report to.
    #[arg(long, value_name = "PATH")]
    pub junit_out_folder: Option<String>,
}

impl CustomTckOpts {
    pub(crate) fn get_junit_out_file(&self, tck_test_name: &str) -> Option<File> {
        self.junit_out_folder.as_ref().map(|path| {
            fs::File::create(format!("{}/tck-{}-results.xml", path, tck_test_name))
                .expect("failed to create JUnit report file")
        })
    }
}
