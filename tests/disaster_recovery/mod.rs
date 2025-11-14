// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Disaster recovery test suite.
//!
//! This module contains automated tests for validating disaster recovery
//! capabilities, measuring RTO/RPO, and ensuring zero data loss.

pub mod database_failure;
pub mod application_crash;
pub mod network_partition;
pub mod data_corruption;
pub mod backup_restore;
pub mod failover;

// Common utilities for DR tests
pub mod common;
