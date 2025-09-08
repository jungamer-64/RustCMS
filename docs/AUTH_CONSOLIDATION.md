# Authentication Consolidation Changelog

## Biscuit Authentication Unification

**Date**: 2025-01-08  
**Version**: 2.0.0  

### Summary
Successfully consolidated all authentication mechanisms to use Biscuit authentication exclusively, removing legacy admin token authentication while maintaining full functionality.

### Changes Made

#### 1. Admin Authentication Migration
- **Before**: Admin endpoints (`/api/v1/admin/*`) used `x-admin-token` header authentication
- **After**: Admin endpoints now use standard Biscuit authentication with role-based permissions
- **Security**: Only users with "admin" permission (SuperAdmin role) can access admin endpoints

#### 2. Code Changes
- **Routes**: Updated admin routes to use `auth_middleware` instead of `admin_auth_layer`
- **Handlers**: Added `require_admin_permission()` checks in admin handlers:
  - `list_posts()`, `create_post()`, `delete_post()`
- **Deprecation**: Marked admin token functions as deprecated with clear migration messages
- **OpenAPI**: Updated security definition from "JWT" to "Biscuit" bearer format

#### 3. Permission System
- **SuperAdmin Role**: Has "admin" permission (can access admin endpoints)
- **Admin Role**: No "admin" permission by default (cannot access admin endpoints)
- **Editor/Author Roles**: No "admin" permission (cannot access admin endpoints)

### Migration Guide

#### For API Consumers
- **Change**: Replace `x-admin-token` header with standard `Authorization: Bearer <biscuit_token>`
- **Requirements**: User must have SuperAdmin role to access admin endpoints
- **Endpoints**: All admin endpoint URLs remain the same (`/api/v1/admin/*`)

#### Example
```bash
# Before (deprecated)
curl -H "x-admin-token: your_admin_token" \
     http://localhost:3000/api/v1/admin/posts

# After (current)
curl -H "Authorization: Bearer <biscuit_token>" \
     http://localhost:3000/api/v1/admin/posts
```

### Security Improvements
1. **Unified Authentication**: Single authentication mechanism across entire API
2. **Role-based Access**: Fine-grained permission control based on user roles
3. **Token Capabilities**: Biscuit tokens provide better security features than simple tokens
4. **Audit Trail**: Better tracking and logging through unified auth system

### Backward Compatibility

- Legacy admin token path removed entirely (`check_admin_token()`, `get_admin_token()`, `admin_auth_layer`).
- `ADMIN_TOKEN` environment variable is ignored if present.

### Testing
- ✅ All existing tests pass
- ✅ New admin permission tests added and passing
- ✅ No functional regressions identified
- ✅ Build system works correctly with deprecation warnings

### Future Considerations
- Consider removing deprecated functions in next major version
- Monitor for any missed admin token usage patterns
- Potential to extend role-based permissions for more granular access control

---

This consolidation achieves the goal of unifying all authentication to Biscuit while making minimal changes and maintaining full API compatibility.