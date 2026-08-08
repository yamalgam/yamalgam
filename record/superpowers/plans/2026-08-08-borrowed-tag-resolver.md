# Borrowed Tag Resolver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let synchronous composer entry points accept tag resolvers that borrow caller-owned state, without changing existing call sites or adding runtime indirection.

**Architecture:** Parameterize `Composer` over its tag resolver type with `TagResolution` as the default generic argument. Default/configured constructors continue to produce the built-in resolver type; custom synchronous constructors instantiate `Composer` directly with the caller's concrete resolver. The entire compose operation remains within the call, so no resolver lifetime needs to escape.

**Tech Stack:** Rust 2024, `yamalgam_core::TagResolver`, `cargo nextest`, Just.

## Global Constraints

- Preserve existing `Composer<'input, I>` source syntax through a default resolver type parameter.
- Preserve every public constructor name, return type, resource-limit policy, and `Value` result.
- Remove the unnecessary `'static` bounds from both bounded and unchecked custom tag-resolver entry points.
- Keep custom resolver state synchronous; do not add ownership wrappers, `Arc`, or third-party dependencies.
- Do not edit immutable audit source artifacts.
- Follow RED -> GREEN, then run the full composer suite and `just check`.

---

### Task 1: Prove borrowed resolver state is currently rejected

**Files:**

Modify `crates/yamalgam-compose/src/lib.rs` tests.

- [ ] **Step 1: Add a borrowed-state resolver regression**

Define a resolver that holds `&HashMap<&str, Value>`. Build the map inside the test, pass the borrowing resolver to `Composer::from_str_with_tag_resolver`, and assert the mapped scalar is returned. Exercise `from_str_with_tag_resolver_unchecked` with the same borrowed state so both public signatures are covered.

- [ ] **Step 2: Run the focused test to verify RED**

```bash
cargo nextest run -p yamalgam-compose borrowed_tag_resolver_state
```

Expected: compilation fails because both constructors require `TagResolver + 'static`.

---

### Task 2: Store the concrete tag resolver in Composer

**Files:**

Modify `crates/yamalgam-compose/src/lib.rs:8-540`.

- [ ] **Step 1: Add the defaulted resolver parameter**

Import `TagResolution` and change the public type to `Composer<'input, I, T = TagResolution>`, with `T: TagResolver`. Replace `Box<dyn TagResolver>` with `T`.

- [ ] **Step 2: Separate built-in construction from generic composition**

Keep `new`, `new_unchecked`, and `new_with_config` on `Composer<..., TagResolution>` and initialize the field from `config.tag_resolution`. Move event consumption and composition methods to a generic `impl<I, T>` constrained by `T: TagResolver`.

Add one private generic constructor that accepts events, a concrete tag resolver, and `LoaderConfig`; it initializes the same limits and counters as the built-in constructor.

- [ ] **Step 3: Remove static bounds and boxing**

Make the bounded, unchecked, and shared custom tag-resolver helpers generic over `T: TagResolver` with no `'static` bound. Construct the generic composer with the private constructor and compose synchronously. Keep event resolver constructors on the default `TagResolution` specialization.

- [ ] **Step 4: Verify GREEN and compatibility**

```bash
cargo nextest run -p yamalgam-compose borrowed_tag_resolver_state
cargo nextest run -p yamalgam-compose
just clippy
```

Expected: borrowed resolver tests compile and pass; all existing composer/default/config/resolver tests remain green with no warnings.

---

### Task 3: Verify, review, and close the finding

- [ ] **Step 1: Run repository gates**

```bash
just check
git --no-pager diff --check
```

- [ ] **Step 2: Request independent review**

Review public generic compatibility, constructor specialization and inference, bounded versus unchecked policy, borrowed lifetime containment, tag-resolution behavior, and absence of unnecessary allocation. Address every confirmed issue and rerun affected gates.

- [ ] **Step 3: Commit code, then append the immutable ledger**

Create a signed focused code commit. Record its exact full SHA in a new bottom entry of `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`, update counters from fixed 14/open 13 to fixed 15/open 12, independently review the append, and create a separate signed ledger commit.
