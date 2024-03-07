/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
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

mod uattributesvalidator;
mod upriority;

pub use uattributesvalidator::*;
pub use upriority::*;

pub use crate::up_core_api::uattributes::*;

#[derive(Debug)]
pub enum UAttributesError {
    ValidationError(String),
    ParsingError(String),
}

impl UAttributesError {
    pub fn validation_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ValidationError(message.into())
    }

    pub fn parsing_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ParsingError(message.into())
    }
}

impl std::fmt::Display for UAttributesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationError(e) => f.write_fmt(format_args!("Validation failure: {}", e)),
            Self::ParsingError(e) => f.write_fmt(format_args!("Parsing error: {}", e)),
        }
    }
}

impl std::error::Error for UAttributesError {}
