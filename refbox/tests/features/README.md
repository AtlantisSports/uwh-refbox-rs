# Feature Specifications

This directory contains Gherkin-style (`.feature`) specifications of
observable behavior in the refbox crate. Each file describes a feature
in plain-English scenarios, following the same style used in
`uwh-portal` (Reqnroll/Cucumber).

## Status: documentation-only

These files are currently **documentation**, not runnable tests. The
Rust `cucumber` crate that would turn them into executable tests has
not yet been wired up in this workspace.

They are committed alongside each feature so that:

1. The intended behavior of each feature is recorded in one place, in
   language a tournament organiser can read without knowing Rust.
2. They are ready to become runnable tests when the cucumber harness
   is added in a later release.

## When these become runnable

Standing up the cucumber harness (adding the dev-dependency, the test
runner, and step definitions) is tracked as a post-v0.4.0 follow-up.
For UI-layout scenarios (see `manual_alarm.feature`), additional UI
testing infrastructure will also be needed, since this crate currently
has no UI-layer tests.
