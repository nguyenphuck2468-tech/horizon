# Horizon Next — Glass Experience Redesign

## Mục tiêu

Nâng cấp trải nghiệm launcher theo thiết kế Glass, giữ nguyên 100% tính năng hiện có, không làm lệch luồng nghiệp vụ. Mỗi tính năng có một nơi quản lý chính, có thể truy cập nhanh từ những nơi cần dùng.

## Phạm vi

| Khu vực | Code hiện có | Hướng xử lý |
|---------|-------------|-------------|
| Glass UI | `glass.rs`, `glass_ui.rs` | Tái sử dụng |
| Account | `account.rs`, `glass_account.rs` | Mở rộng thành trang riêng (Account Center) |
| Account switcher | `switch_page.rs` | Nâng cấp và tái sử dụng |
| Home | `glass_home.rs`, `glass_home_tabs.rs` | Cải thiện trải nghiệm |
| Mods | `mods_page.rs`, `glass_mods.rs` | Mở rộng chức năng |
| Servers | `servers_page.rs`, `glass_servers.rs` | Cải thiện điều hướng |
| Settings | `glass_settings.rs` | Phân chia lại trách nhiệm |
| Instances | `instances_page.rs` | Kết nối với Home và launch profile |
| Skin/Wardrobe | `skin_page.rs`, `wardrobe_page.rs` | Tích hợp với Account Center |

## Nguyên tắc thiết kế

1. **Không viết lại luồng nghiệp vụ**: UI chỉ gọi workflows/services hiện có.
2. **UI chỉ nhận trạng thái an toàn**: không nhận access token/refresh token.
3. **Ely.by giữ trạng thái ownership riêng**: không biến `owns: false` thành `true`.
4. **Offline identity không giả thành Microsoft account**.
5. **Mỗi tính năng có một nơi quản lý chính**: Account được quản lý ở Account Center, Home chỉ có bộ chọn nhanh.

## Cấu trúc file mục tiêu

```
launcher/src/
├── main.rs
├── glass.rs
├── glass_ui.rs
│
├── accounts_page.rs          # Account Center controller
├── glass_accounts.rs         # Account Center layout
├── account_details.rs        # Account detail panels
├── account_status.rs         # Safe status presentation
├── account.rs                # Existing auth/backend logic
├── switch_page.rs            # Top-bar quick switch
│
├── glass_home.rs
├── glass_home_tabs.rs
├── glass_home_lists.rs
├── launch_profile.rs         # New
├── launch_readiness.rs       # New
│
├── mods_page.rs
├── glass_mods.rs
├── mod_compatibility.rs      # New
│
├── servers_page.rs
├── glass_servers.rs
│
├── instances_page.rs
├── glass_settings.rs
├── settings_sections.rs      # New
│
├── skin_page.rs
└── wardrobe_page.rs
```

## Kế hoạch commit (25 commit, 5 phase)

### Phase 0 — Chuẩn bị (C00-C02)

| Commit | Nội dung |
|--------|----------|
| C00 | `docs: define glass experience redesign` — tài liệu này |
| C01 | `test: add launcher navigation and account regression coverage` |
| C02 | `refactor: centralize glass page navigation metadata` |

### Phase 1 — Account Center (C03-C10)

| Commit | Nội dung |
|--------|----------|
| C03 | `feat(ui): add accounts page navigation` |
| C04 | `feat(ui): build glass account center overview` |
| C05 | `feat(ui): add unified account list and provider cards` |
| C06 | `feat(ui): add account details and status summaries` |
| C07 | `feat(ui): add account security and reauthentication states` |
| C08 | `feat(ui): integrate skin management into account center` |
| C09 | `feat(ui): upgrade account switcher popover` |
| C10 | `refactor(ui): reduce settings account section to summary` |

### Phase 2 — Home và Launch Experience (C11-C15)

| Commit | Nội dung |
|--------|----------|
| C11 | `feat(ui): add launch readiness panel` |
| C12 | `feat(ui): add recent instance launcher` |
| C13 | `feat(ui): add launch profile selector` |
| C14 | `feat(ui): add instance health indicators` |
| C15 | `feat(ui): add recent launcher activity` |

### Phase 3 — Mods và Servers (C16-C20)

| Commit | Nội dung |
|--------|----------|
| C16 | `feat(mods): add dependency and compatibility details` |
| C17 | `feat(mods): add update preview and batch actions` |
| C18 | `feat(mods): add safe rollback and profile backup` |
| C19 | `feat(servers): add favorites and recent servers` |
| C20 | `feat(servers): add server compatibility indicators` |

### Phase 4 — Settings, Polish và Release (C21-C25)

| Commit | Nội dung |
|--------|----------|
| C21 | `refactor(settings): separate global and instance preferences` |
| C22 | `feat(settings): add storage and diagnostics panels` |
| C23 | `ui: polish glass responsiveness and accessibility` |
| C24 | `perf(ui): optimize glass rendering and image caching` |
| C25 | `test: add launcher visual regression and release checks` |

## Quy tắc Git

1. Tạo nhánh `horizon-next/glass-experience` từ Phase 3.
2. Mỗi commit chỉ thay đổi một nhóm chức năng.
3. Không sửa `main` trực tiếp.
4. Sau mỗi checkpoint chạy test và tạo PR draft để review.
5. Chỉ merge khi CI và kiểm thử thủ công đạt yêu cầu.
6. Không squash toàn bộ thành một commit lớn.

## Lưu ý Ely.by

- Giữ luồng OAuth2 qua trình duyệt và refresh token theo scope đã cấp.
- Ely.by yêu cầu `offline_access` nếu ứng dụng muốn nhận refresh token.
- Không mở rộng scope về sau mà coi là thao tác làm mới token thông thường.
