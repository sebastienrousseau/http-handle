# Enterprise Authorization (RBAC/ABAC)

The enterprise module provides pluggable authorization primitives:

- `AuthorizationEngine` trait
- `RbacAdapter` for role-permission mapping
- `AbacAdapter` for attribute-based access control rules
- `AuthorizationHook` for chained policy evaluation

## RBAC

RBAC grants permissions by role:

- Subject -> roles
- Role -> `(resource, action)` tuples

Use `RbacAdapter::grant_role` and `RbacAdapter::grant_permission`.

## ABAC

ABAC rules are matched by `(resource, action)` and validated against required
attributes.

Use `AbacAdapter::with_rule` and define `AbacRule.required_attributes`.

## Hook Chaining

`AuthorizationHook` evaluates engines in order and short-circuits on first deny.

This allows combining coarse RBAC and fine ABAC checks in a single request
pipeline.
