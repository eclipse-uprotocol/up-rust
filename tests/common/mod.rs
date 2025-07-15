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

use std::{
    fs::{self, File},
    str::FromStr,
};

use cucumber::{cli, codegen::WorldInventory, parser::basic::Walker, World};

#[allow(dead_code)]
const FEATURES_PATH: &str = "";

#[allow(dead_code)]
#[derive(cli::Args)]
struct CustomTckOpts {
    /// The folder to write the JUnit report to.
    #[arg(long, value_name = "PATH")]
    pub junit_out_folder: Option<String>,
}

impl CustomTckOpts {
    #[allow(dead_code)]
    pub(crate) fn get_junit_out_file(&self, tck_test_name: &str) -> Option<File> {
        self.junit_out_folder.as_ref().map(|path| {
            fs::File::create(format!("{path}/tck-{tck_test_name}-results.xml"))
                .expect("failed to create JUnit report file")
        })
    }
}

/// Runs Cucumber based tests using a given World.
///
/// # Arguments
///
/// * `features_glob_pattern` - A glob pattern that is used to determine the Gherkin
///   feature files containing the scenarios to run.
/// * `junit_result_file_name_prefix` - The prefix to include in the JUnit result file
#[allow(dead_code)]
pub(crate) async fn run<T: World + WorldInventory + std::fmt::Debug>(
    features_glob_pattern: &str,
    test_name_prefix: &str,
) -> Result<(), std::io::Error> {
    let walker = Walker::from_str(features_glob_pattern)?;

    let custom_opts = cli::Opts::<
        cucumber::parser::basic::Cli,
        cucumber::runner::basic::Cli,
        cucumber::writer::basic::Cli,
        CustomTckOpts,
    >::parsed();

    if let Some(file) = custom_opts.custom.get_junit_out_file(test_name_prefix) {
        let mut opts = cli::Opts::<
            cucumber::parser::basic::Cli,
            cucumber::runner::basic::Cli,
            cucumber::writer::junit::Cli,
            CustomTckOpts,
        >::parsed();
        opts.parser.features = Some(walker);
        T::cucumber()
            .with_writer(cucumber::writer::JUnit::new(file, 0))
            .with_cli(opts)
            .fail_on_skipped()
            .run(FEATURES_PATH)
            .await;
    } else {
        let mut opts = cli::Opts::<cucumber::parser::basic::Cli, _, _, CustomTckOpts>::parsed();
        opts.parser.features = Some(walker);
        T::cucumber()
            .with_cli(opts)
            .fail_on_skipped()
            .run(FEATURES_PATH)
            .await;
    }
    Ok(())
}

/// Strips the `0x` or `0X` prefix from a hex string.
#[allow(dead_code)]
pub(crate) fn trimhex(s: &str) -> &str {
    s.strip_prefix("0x")
        .unwrap_or(s.strip_prefix("0X").unwrap_or(s))
}

/// Tries to parse a string into an unsigned integer.
///
/// # Panics
///
/// if the given string is neither a hex nor decimal string representation of
/// the expected unsigned integer type.
#[allow(dead_code)]
pub(crate) fn value_as_u8(value: String) -> u8 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u8::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

/// Tries to parse a string into an unsigned integer.
///
/// # Panics
///
/// if the given string is neither a hex nor decimal string representation of
/// the expected unsigned integer type.
#[allow(dead_code)]
pub(crate) fn value_as_u16(value: String) -> u16 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u16::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

/// Tries to parse a string into an unsigned integer.
///
/// # Panics
///
/// if the given string is neither a hex nor decimal string representation of
/// the expected unsigned integer type.
#[allow(dead_code)]
pub(crate) fn value_as_u32(value: String) -> u32 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u32::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

/// Tries to parse a string into an unsigned integer.
///
/// # Panics
///
/// if the given string is neither a hex nor decimal string representation of
/// the expected unsigned integer type.
#[allow(dead_code)]
pub(crate) fn value_as_u64(value: String) -> u64 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u64::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}
