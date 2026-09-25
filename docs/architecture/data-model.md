# Data model

SQLite, created by `crates/db/src/migration`. Foreign keys are enforced (`PRAGMA foreign_keys = ON` in `connect`) and cascade on delete. All timestamps are UTC.

```
user 1───* session
  │ 1───* passkey
  │ 1───1 user_settings
  │ 1───* category 1───* job 1───* job_run
  └ 1───* job
```

| Table | Columns |
|---|---|
| `user` | id, username (unique, lowercase), full_name, password_hash, created_at |
| `session` | token_hash (PK, SHA-256 hex), user_id, expires_at |
| `passkey` | id, user_id, credential_id (unique), credential (JSON), label, created_at |
| `user_settings` | user_id (PK), theme, tab_orientation, job_layout, date_format, time_format, page_size, language |
| `category` | id, user_id, name. Unique (user_id, name); matching is case-insensitive in the service layer |
| `job` | id, user_id, category_id, title, summary, description, first_run, repeat_kind, repeat_value, next_run, created_at |
| `job_run` | id, job_id, scheduled_for, started_at, finished_at, status (`running`/`success`/`failed`), message |

## Notes

- `Default` category is created per user at registration. Deleting a category moves its jobs to `Default`; `Default` itself cannot be deleted.
- `repeat_kind`/`repeat_value` encode `Repeat` (`seconds|minutes|days|weeks|months|weekday|day_of_month`; weekday 0=Mon..6=Sun). Both NULL means no repeat.
- `next_run` NULL means nothing further is scheduled (a non-repeating job that has run). It is indexed for the scheduler.
- Enumerated settings are stored as strings and parsed leniently, falling back to defaults.
- Category `unique(user_id, name)` is case-sensitive at the DB level; case-insensitive uniqueness is enforced by `get_or_create`.
