# EasyInventory Generalization V1.7-V1.9 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the V1.7-V1.9 generalization PRD so EasyInventory is no longer tied to 科展商行 and can be configured for generic single-machine trade, wholesale, and delivery businesses.

**Architecture:** Keep the existing SQLite and Tauri command architecture stable. Add a configuration layer for merchant profile, terminology, industry templates, feature flags, document defaults, and generic import flows while preserving the current 科展-compatible Excel importer as an advanced legacy path.

**Tech Stack:** Tauri 2, Rust, rusqlite, calamine, umya-spreadsheet, React 19, TypeScript, Ant Design, Vite.

---

## File Map

- `src-tauri/src/db.rs`: seed generic default settings, expose reusable setting helpers, keep guest customer compatibility.
- `src-tauri/src/models.rs`: add DTOs and requests for setup status, merchant profile, terms, industry templates, document templates, feature flags, and generic imports.
- `src-tauri/src/commands.rs`: add setup/config/template/import commands and unit tests.
- `src-tauri/src/reports.rs`: replace 科展商行 runtime defaults with merchant-aware generic defaults.
- `src-tauri/src/lib.rs`: register new Tauri commands.
- `src/shared/types.ts`: add matching frontend types.
- `src/api/inventory.ts`: expose new API helpers.
- `src/store/appStore.ts`: store settings, terms, feature flags, setup status.
- `src/App.tsx`: load settings/terms, apply feature switches, route empty setup to `/setup`.
- `src/pages/SetupPage.tsx`: first-use setup wizard.
- `src/pages/SettingsPage.tsx`: add merchant profile, terms, industry templates, generic imports, and advanced legacy migration.
- `scripts/robustness-check.mjs`: add checks for V1.7-V1.9 gates.
- `tests/e2e/core-flows.spec.ts`: add browser coverage for setup/config/import path.

## Task 1: Generic Runtime Defaults

- [ ] Add failing Rust tests proving fresh settings and default order templates do not contain 科展商行.
- [ ] Change `seed_settings` and report template defaults to generic values.
- [ ] Add settings keys for setup, merchant profile, terminology, feature flags, and active template.
- [ ] Run focused Rust tests.

## Task 2: Merchant, Terms, Setup, and Industry Template Commands

- [ ] Add models for `SetupStatusDto`, `MerchantProfileDto`, `TermSettingsDto`, `IndustryTemplateDto`, `SetupRequest`, and feature flags.
- [ ] Add commands: `get_setup_status`, `complete_setup`, `get_merchant_profile`, `save_merchant_profile`, `get_term_settings`, `save_term_settings`, `list_industry_templates`, `apply_industry_template`.
- [ ] Add tests for setup completion, merchant persistence, term persistence, and industry template application.
- [ ] Register commands in `lib.rs`.

## Task 3: Generic Import Preview and Confirm

- [ ] Add models for `GenericImportRequest`, `GenericImportPreviewDto`, `GenericImportRowDto`, `GenericImportResultDto`.
- [ ] Implement generic product, customer, and initial-stock import preview.
- [ ] Implement confirm import with duplicate strategies `skip` and `overwrite`.
- [ ] Preserve current `import_excel` as legacy-compatible full migration and require backup before it clears data.
- [ ] Add tests proving generic imports do not clear orders and legacy import is separate.

## Task 4: Frontend Setup and Settings UX

- [ ] Add frontend types and API wrappers.
- [ ] Add setup wizard route with merchant, template, terms, document, and import steps.
- [ ] Extend settings page with merchant profile, terms, industry templates, generic import, and advanced legacy migration sections.
- [ ] Update app shell to use merchant name and feature flags.
- [ ] Keep existing pages functional if settings fail to load.

## Task 5: V1.7-V1.9 Verification

- [ ] Add robustness checks for no runtime 科展 defaults, generic import commands, setup route, and industry templates.
- [ ] Add browser E2E path for setup/settings genericization.
- [ ] Update README with V1.7-V1.9 status after implementation.
- [ ] Run `npm run check`.
- [ ] Run `npm run tauri:build` and `npm run package:smoke` before completion.
