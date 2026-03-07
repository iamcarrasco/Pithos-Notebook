/// Built-in templates tailored for Identity Architects, Security SMEs,
/// and Enterprise Architects.
pub fn builtin_templates() -> Vec<(String, String, String)> {
    vec![
        (
            "Threat Model".into(),
            r#"# Threat Model

## System Overview

**System name**:
**Owner**:
**Date**:

## Assets

| Asset | Classification | Location |
| --- | --- | --- |
|  |  |  |

## Threat Actors

-

## Attack Surface

| Entry Point | Protocol | Authentication |
| --- | --- | --- |
|  |  |  |

## Threats (STRIDE)

| ID | Category | Threat | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- | --- |
| T-001 |  |  |  |  |  |

## Mitigations

- [ ]

## Residual Risks

-

## Review History

| Date | Reviewer | Notes |
| --- | --- | --- |
|  |  |  |
"#.into(),
            "security,threat-model".into(),
        ),
        (
            "Architecture Decision Record".into(),
            r#"# ADR-NNN: [Title]

## Status

Proposed | Accepted | Deprecated | Superseded by ADR-XXX

## Context

Describe the forces at play, including technical, political, social, and project constraints.

## Decision

State the decision that was made.

## Consequences

Describe the resulting context after applying the decision. List both positive and negative consequences.

## Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
|  |  |  |

## References

-
"#.into(),
            "architecture,adr".into(),
        ),
        (
            "IAM Blueprint".into(),
            r#"# IAM Blueprint

## Scope

**Environment**:
**Identity Provider**:
**Date**:

## Identity Lifecycle

| Phase | Process | Owner | SLA |
| --- | --- | --- | --- |
| Joiner |  |  |  |
| Mover |  |  |  |
| Leaver |  |  |  |

## Role Hierarchy

```mermaid
graph TD
    A[Global Admin] --> B[Tenant Admin]
    B --> C[Security Admin]
    B --> D[Application Admin]
```

## Access Policies

| Policy | Scope | Conditions | Grant |
| --- | --- | --- | --- |
|  |  |  |  |

## Privileged Access

- [ ] Just-in-time elevation configured
- [ ] Break-glass accounts documented
- [ ] PAM solution integrated

## Audit & Compliance

-
"#.into(),
            "iam,identity,security".into(),
        ),
        (
            "Runbook".into(),
            r#"# Runbook: [Procedure Name]

## Overview

**Purpose**:
**Owner**:
**Last tested**:
**Estimated duration**:

## Prerequisites

- [ ]

## Procedure

### Step 1 —

```bash

```

### Step 2 —

```bash

```

### Step 3 —

```bash

```

## Verification

- [ ]

## Rollback

1.

## Contacts

| Role | Name | Contact |
| --- | --- | --- |
| Primary |  |  |
| Escalation |  |  |
"#.into(),
            "runbook,operations".into(),
        ),
        (
            "Meeting Notes".into(),
            "# Meeting Notes\n\n**Date**: \n**Attendees**:\n\n## Agenda\n\n1. \n\n## Discussion\n\n- \n\n## Action Items\n\n- [ ] \n".into(),
            "meeting".into(),
        ),
        (
            "Security Review".into(),
            r#"# Security Review

## Application

**Name**:
**Version**:
**Review date**:
**Reviewer**:

## Scope

-

## Findings

| ID | Severity | Title | Status |
| --- | --- | --- | --- |
| F-001 | Critical / High / Medium / Low |  | Open |

## F-001: [Finding Title]

**Severity**:
**Component**:
**Description**:

**Recommendation**:

**Evidence**:

```

```

## Summary

| Severity | Count |
| --- | --- |
| Critical | 0 |
| High | 0 |
| Medium | 0 |
| Low | 0 |

## Sign-off

- [ ] Findings reviewed with development team
- [ ] Remediation timeline agreed
"#.into(),
            "security,review".into(),
        ),
    ]
}
