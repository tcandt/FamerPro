# FARMERPRO — MASTER PRODUCT, UI/UX & RUST ENGINE BLUEPRINT

> **Phiên bản:** 1.0 · **Ngày:** 20/11/2026 · **Trạng thái:** THIẾT KẾ/ĐỀ XUẤT — chưa triển khai, chưa benchmark  
> **Sản phẩm:** FarmerPro — Android Workflow Automation Studio  
> **Bàn giao mục tiêu:** Ứng dụng Windows GUI đóng gói thành `FarmerPro.exe` và bộ cài `FarmerPro_*-setup.exe` / `.msi`; core điều khiển viết bằng Rust; giao diện kéo-thả kiểu n8n với phong cách dark-premium.  
> **Nguồn tham chiếu thị giác:** ảnh “AutoFarm Workflow Studio” được người dùng cung cấp ngày 20/11/2026.  
> **Nguyên tắc:** tái thiết kế từ đầu; chỉ giữ tinh thần màu navy/xanh lam; không sao chép bố cục quá chật, không nhầm ảnh demo với ứng dụng đã chạy.

---

## 0. Tuyên bố sản phẩm & quyết định quan trọng

**FarmerPro không phải bản sao n8n.** Sản phẩm là phần mềm desktop để người dùng mô hình hóa các thao tác điều khiển thiết bị Android thành **workflow đồ thị có điều kiện, quan sát trạng thái, phát hiện mục tiêu, xác minh kết quả, retry, checkpoint và quản lý nhiều thiết bị**. n8n là tham chiếu về trải nghiệm xây dựng luồng, không phải runtime/giao thức Android.

**Kiến trúc được chọn:**

- **Rust 2024 edition + Tokio**: lõi nghiệp vụ, device drivers, workflow interpreter, scheduler, persistence, chính sách thao tác, OCR orchestration và native services.
- **Tauri 2**: desktop shell Windows, IPC phân quyền. Tauri không tương đương GUI 100% Rust; frontend là WebView2.
- **Vite + React + TypeScript**: UI/UX, không nhúng Node runtime trong bản release (Node dùng khi build).
- **`@xyflow/react` (React Flow)**: node graph canvas, port/edge, zoom/pan, custom node, validation; khóa phiên bản và kiểm tra API khi cài vì v13 đã phát hành 16/11/2026 và có thay đổi.
- **Tailwind CSS v4 + shadcn/ui (Radix) + Lucide**: các primitive và component tự sở hữu, tinh chỉnh bằng FarmerPro tokens; không pha trộn nhiều hệ design system.
- **Zustand** cho state editor; **TanStack Query** cho server/device data; **Zod** validation phía UI; **Serde/JSON Schema** phía Rust.
- **SQLite WAL + migrations** cho MVP local-first. PostgreSQL chỉ khi cần nhiều Gateway/host hoặc hệ điều phối phân tán.
- **ADB USB/TCP** cho MVP; Android 9–13; Android APK Agent/WebRTC để dành phiên bản sau. **Không làm video streaming tại MVP.**
- **StarCi Harness** là *harness quản lý AI coding và quality gates*, không phải runtime AutoFarm. Sử dụng như công cụ bên ngoài sau khi đã đọc tài liệu và cấu hình phù hợp; repository FarmerPro phải độc lập.

**Chính sách triển khai:** không Docker bắt buộc; không yêu cầu đăng nhập cloud để thao tác ADB cục bộ; không hứa hỗ trợ mọi game/thiết bị khi chưa thử nghiệm; không tự ý xóa app data hay thay proxy; những thao tác có rủi ro cần xác nhận, ghi log và quyền phù hợp.

---

## 1. Product Vision, Jobs-to-be-Done và tiêu chí thành công

### 1.1 Người dùng chính

1. **Operator:** kết nối điện thoại, chọn thiết bị/tài khoản, chạy nhiệm vụ, theo dõi lỗi, pause/resume/stop.
2. **Workflow Creator:** tạo/kéo-thả node, chọn mục tiêu từ ảnh, cấu hình biến, branch/loop, chạy thử từng bước.
3. **Administrator:** cấu hình gateway, tài khoản, thiết bị, proxy, backup, audit, phân quyền.
4. **Developer/QA:** xem traces, screenshots ở điểm xác minh, export run report, mô phỏng/hồi quy và kiểm tra package.

### 1.2 Những công việc cốt lõi

- “Tôi muốn tạo quy trình **không cần viết code** bằng node, xem kết quả của từng node và chỉnh lỗi trong cùng cửa sổ.”
- “Tôi muốn chọn các nhiệm vụ như *tân thủ, đánh quái, nhặt vật phẩm, nhận thưởng*, engine xét tính khả dụng và chạy tuần tự cho từng thiết bị.”
- “Tôi muốn lệnh click **tính từ vùng nhận diện**, không click một điểm cố định dùng chung nhiều máy.”
- “Tôi muốn điều phối 20–50 điện thoại nhưng **mỗi máy một action stream tuần tự**, job/account không tranh chấp.”
- “Tôi muốn đóng gói **Windows EXE**, chạy offline cho ADB local, có cập nhật/báo cáo và không cần mở trình duyệt.”

### 1.3 Các KPI đề xuất (chỉ là mục tiêu test, chưa phải thành tích)

| Mục | Mục tiêu nghiệm thu | Điều kiện đo |
|---|---|---|
| M1 — CLI ADB | 100 lần gọi tap/swipe test có ghi phản hồi | 1 thiết bị, test app có kiểm soát |
| M2 — Vision | Precision ≥ 98%, recall và false-click được báo cáo | bộ ảnh có ground truth theo máy/độ phân giải |
| M3 — Safe tap | 100% điểm mẫu nằm trong mask hợp lệ, ≤ số phép thử lỗi đã định | property tests + screenshots |
| M4 — Workflow | ≥ 95% hoàn thành luồng mẫu | tối thiểu 100 runs test app, nêu điều kiện mạng |
| M5 — Multi-device | 5 → 20 → 50 thiết bị; không có device double-lease | soak 2h/8h/24h, theo từng mốc |
| GUI responsiveness | thao tác canvas thông thường p95 ≤ 100 ms (mục tiêu) | Windows test hardware được ghi |
| Packaged Windows | smoke test trên Windows 11 sạch | cài, chạy, thêm máy, chạy workflow, gỡ cài |

Đối với game cụ thể, độ chính xác OCR, thời gian chạy và giới hạn thao tác phải đo trên chính game đó; con số trên không phải bảo đảm.

---

## 2. Phạm vi theo phiên bản — tránh ôm đồm

### MVP V0.1–V0.3 — phải hoàn thành trước

- Kết nối ADB USB, TCP cục bộ đã xác thực; liệt kê máy, serial, kích thước/orientation, trạng thái.
- Device capture theo yêu cầu (PNG), không live video.
- Node workflow: Start, End, Launch App, Capture, Find Image, OCR Text (hoặc fallback capability), Random Safe Tap, Swipe Region, Wait, IF/ELSE, Retry, Loop có giới hạn, Verify, Checkpoint.
- UI Studio: editor đồ thị, node palette, inspector, debug dock, run/pause/stop từng step.
- Nhiệm vụ có thứ tự/điều kiện; SQLite lưu workflow, run, step events, asset templates.
- Tauri Windows build, NSIS setup EXE; kiểm thử ít nhất 3 Android 9–13 khác hãng/độ phân giải.

### V0.4–V0.6 — mở rộng sau khi MVP đạt

- 5 → 20 → 50 device actors, queue độc lập, bounded Vision workers.
- Task catalog, task dependencies, availability rules, profile theo game, onboarding nhiều account.
- Network profile/proxy theo quyền thiết bị; account switch flow có kiểm soát, audit và manual approval khi cần.
- Export/import workflow versioned, shared templates, diagnostics bundle, backup.

### V1.x — tương lai, không thực hiện ở MVP

- Android APK Agent kết nối từ xa và MediaProjection/Accessibility với người dùng cấp quyền.
- Streaming video/WebRTC/cloud relay, quản lý gateway nhiều host và PostgreSQL.
- API game trực tiếp **chỉ khi có tài liệu/ủy quyền**; không thiết kế bypass xác thực, chống gian lận hoặc request ẩn trái phép.
- Online marketplace/template sharing và auto-update ký số.

---

## 3. Art Direction — không copy layout demo

### 3.1 Tham chiếu đã quan sát trực tiếp

Ảnh tham chiếu có nền chủ đạo gần `#111822`, thanh bên `#161E29`, vùng canvas `#131823`, thanh trên `#19212E`, điểm nhấn xanh khoảng `#0D79FD`. Đây là **mẫu màu lấy từ một số pixel trong ảnh**, không phải bảng màu chính thức do tác giả công bố.

**Giữ:** navy sâu, độ tương phản tốt, xanh electric có chủ đích, badge trạng thái sạch, node viền tinh tế, nhóm chức năng rõ ràng.

**Thiết kế lại:** giảm ba tầng sidebar chật; không đặt palette 200px + canvas + inspector 330px + logs cao cùng lúc trên màn hình 1366; giảm số đường viền và glow; thay “nút màu ở mọi nơi” bằng hierarchy nghiêm túc; mọi panel có lý do tồn tại; khi không chọn node, inspector chuyển thành helpful empty state.

### 3.2 Design DNA — 8 nguyên tắc bất biến

1. **Precision over decoration:** một hành động chính tại mỗi khu vực; màu chỉ dùng để biểu thị tương tác hoặc trạng thái.
2. **Workstation-grade:** thích hợp làm việc liên tục 6–10 giờ, dễ đọc, không mỏi mắt; bảng dữ liệu dày vừa phải.
3. **Canvas-first:** workflow canvas là trung tâm và được ưu tiên không gian; panel phụ tự thu gọn.
4. **Progressive disclosure:** thao tác đơn giản ngay mặt chính, cài đặt nâng cao trong accordion/sheet.
5. **Stable hierarchy:** sidebar định hướng → header bối cảnh → nội dung → inspector → execution dock.
6. **No visual noise:** không gradient nền, không neon dày, không glassmorphism mọi nơi, không shadow lan rộng.
7. **Verification-first:** UI phân biệt ‘đã gửi lệnh’ và ‘đã xác minh trạng thái’; lỗi có nguyên nhân và đường phục hồi.
8. **Device-aware:** mọi quyết định chọn click dựa trên thiết bị/khung hình hiện tại, không mặc định dùng chung tọa độ.

### 3.3 Màu sắc chuẩn đề xuất

| Token | Hex | Mục đích |
|---|---|---|
| `bg.app` | `#101722` | lớp nền sâu nhất |
| `bg.sidebar` | `#161E29` | thanh điều hướng |
| `bg.surface` | `#192230` | card/toolbar |
| `bg.elevated` | `#202B3B` | popover/inspector |
| `bg.canvas` | `#121A26` | canvas node |
| `stroke.subtle` | `#2A3647` | viền mặc định |
| `stroke.focus` | `#4B9CFF` | focus keyboard/selection |
| `fg.primary` | `#EDF3FC` | tiêu đề/điểm nhấn chữ |
| `fg.secondary` | `#A9B8CC` | mô tả |
| `fg.muted` | `#74869D` | metadata |
| `accent.primary` | `#1878F5` | primary actions |
| `accent.hover` | `#2F8CFF` | hover |
| `accent.soft` | `#173355` | lựa chọn/chip |
| `success` | `#3BC38D` | verified success |
| `warning` | `#E7B15B` | retry/wait |
| `danger` | `#F27883` | stop/error |
| `vision` | `#B291F7` | node vision/OCR |
| `neutral.node` | `#7BA6D6` | node điều khiển |

**Cấm:** gradient toàn màn hình, text neon rực, lạm dụng box đổ bóng, dùng màu đỏ/xanh để phân biệt mà thiếu label/icon, icon không thống nhất. Tỷ lệ accent chỉ nên xuất hiện tại focus/primary/selected/important status, không tràn khắp bảng.

### 3.4 Typography, spacing, icon

- Font giao diện: **Inter** hoặc hệ font Windows `Segoe UI Variable` (font fallback hợp lệ; không yêu cầu đính kèm font độc quyền trong repo).
- Chữ thường 13px/line-height 19px, bảng 12–13px, nhãn metadata 11–12px; headings h1 20–24px, h2 16–18px, tránh quá nhiều cỡ chữ.
- Mặc định tabular numbers với device IDs, durations, x/y, tỷ lệ, FPS và thống kê.
- Spacing hệ 4px: 4/8/12/16/20/24/32. Panel padding 12–16; grid gap 12–16.
- Border-radius: 6px button/input, 9px card/panel, 12px modal; node 10px; hạn chế pill cho mọi button.
- Icon: **Lucide** nét 1.75–2, 16/18/20px; mỗi icon có semantic label/tooltip.
- Shadow: chỉ có cho popover/dialog, rất nhẹ; focus ring 2px có tương phản.
- Motion: 120–180ms micro-interaction; 180–240ms panel transitions, easing ổn định; `prefers-reduced-motion` tắt transitions không cần thiết.

---

## 4. Shell, Header, Sidebar & bố cục nền tảng

### 4.1 Windows frame & design grid

- Target trải nghiệm xuất sắc ở **1600×900**, tương thích **1440×900** và sử dụng được **1280×720**.
- Kích thước tối thiểu đề xuất cho toàn ứng dụng: **1120×700**; nếu khung nhỏ hơn, ẩn context panel và dùng tab/overlay chứ không cắt controls.
- Native Windows title bar hoặc custom Tauri titlebar phải có drag region đúng, keyboard focus và control minimize/maximize/close; không chiếm 50px để trang trí.
- Khung UI: `AppTitlebar(38)` + `AppCommandBar(52)` + phần dưới `NavigationRail(64) + MainWorkspace(flex)`.
- Sidebar mở rộng `220px`, thu gọn `64px`, nhớ preference. Không đặt search rộng trong rail.
- Canvas space phải đạt ít nhất 55–65% vùng chiều ngang ở độ phân giải mục tiêu; Node Library 224–256px và Inspector 288–336px có thể ẩn.
- Execution Dock mặc định `220px`, có resize 160–420px, có `Collapse`, không che canvas.

### 4.2 Shell 4 lớp — lý do và hành vi

```text
┌──────────── Native Window / App Titlebar (38) ─────────────────────────────────┐
│ FarmerPro · workspace name                                     Window controls │
├──────────── Command Bar (52) ───────────────────────────────────────────────────┤
│ Breadcrumb | Workflow name + dirty state | Device target | Undo Redo | Run Stop │
├── Navigation (64/220) ┬──────────────── Workspace content ──────────────────────┤
│ Home                   │ context-specific command / header                       │
│ Devices                │ ┌palette(240)┬────── CANVAS flex ───────┬inspector(312)┐ │
│ Workflows              │ │ search     │ zoom/pan + graph         │ property form │ │
│ Tasks                  │ │ categories │ guides/grid/mini-map    │ help/error    │ │
│ Runs                   │ └────────────┴─────────────────────────┴──────────────┘ │
│ Accounts               │ ┌────── execution dock: events / traces / result ──────┐ │
│ Network                │ └──────────────────────────────────────────────────────┘ │
│ Settings               │ bottom status: ADB | 3 devices | queue | local mode     │
└────────────────────────┴───────────────────────────────────────────────────────┘
```

### 4.3 Sidebar — kiến trúc thông tin chuẩn

**Nhóm vận hành**: Tổng quan (`layout-dashboard`), Thiết bị (`smartphone`), Quy trình (`workflow`), Nhiệm vụ (`list-checks`), Phiên chạy (`activity`).

**Nhóm tài nguyên**: Tài khoản (`users`), Mạng/Proxy (`network`), Kho mẫu (`images`), Nhật ký (`scroll-text`).

**Nhóm hệ thống** (đặt cuối): Cài đặt (`settings`), Trợ giúp (`circle-help`).

- Bỏ nav “Lịch chạy” khỏi nhóm chính ở MVP; đưa lịch vào trang Nhiệm vụ/Jobs dưới dạng tab.
- Rail thu gọn luôn có tooltip và active indicator trái 3px. Nhóm sidebar mở rộng có nhãn rõ, có keyboard Tab, không ẩn nút thao tác thiết yếu.
- Brand tại rail: logo FP dạng khối, không cần chữ chạy dài. Workspace switcher đặt trong header trái.
- Trạng thái kết nối (đã ghép ADB, bản CLI, DB) hiển thị ở thanh status bottom, **không** biến thành một dãy badge ở top mọi trang.

### 4.4 Header / Command Bar

**Left:** breadcrumb `FarmerPro / Quy trình / Nhiệm vụ tân thủ`; cạnh tên: dirty dot và trạng thái draft/published.

**Middle:** global command search `Ctrl+K`; chọn device target (trong Workflow Editor mới xuất hiện); undo `Ctrl+Z`/redo `Ctrl+Shift+Z`.

**Right:** `Validate` (secondary), `Run` (primary), dropdown `Run options`, `Stop` chỉ bật khi đang chạy, notification icon + avatar menu chỉ khi thực sự cần tài khoản sản phẩm; nếu local-only thì dùng biểu tượng profile preferences không bịa login cloud.

- Không đặt 3 nút primary cạnh nhau. Trong Edit `Run` là primary; trong Running `Pause` primary và `Stop` destructive secondary.
- Ctrl+S save, Ctrl+Enter run preview, Esc close inspector modal, Ctrl+K command palette, `?` shortcuts.

### 4.5 Responsive desktop/window

| Width | Nav | Palette | Inspector | Run Dock |
|---|---|---|---|---|
| ≥1600 | 220 expanded | 240 | 312 | split bottom 220 |
| 1440–1599 | 64 rail default | 232 | 304 | bottom 210 |
| 1280–1439 | 64 rail | 216 collapsible | 288 overlay/drawer | bottom 180 |
| 1120–1279 | 64 rail | popover/command palette | drawer overlay | tabbed 160 |
| <1120 | không phải viewport mục tiêu | cảnh báo/layout graceful | không để form bị cắt | — |

---

## 5. Các màn hình bắt buộc — User Journeys & Screen Specs

### 5.1 Onboarding / First Run

- Welcome screen 2 cột cân đối, không quảng cáo giả; status môi trường: Rust core, adb executable, thiết bị, storage, permissions.
- Button `Kiểm tra ADB`, hướng dẫn USB debugging và device authorization; không tự sửa hệ thống một cách bí mật.
- Quy trình: chọn workspace → chọn thư mục dữ liệu → kiểm tra ADB → kết nối thiết bị → tạo workflow mẫu → run dry/mock → run thật.
- Kết quả `Ready / Attention / Blocked` có actionable next step; chọn nhắc lại hoặc skip hỗ trợ UI-only demo.

### 5.2 Overview — Command Center

- Top header: “Tổng quan” + `Run selected tasks` (nếu thiết bị và task hợp lệ).
- 4 KPI ở một hàng: Devices connected, Active jobs, Attention required, Success rate (chỉ số liệu thật; chưa có data hiển thị `—`).
- Body 2 cột 7:5: `Running now` virtualized table, `Needs attention` queue.
- Bottom: `Recent Workflows`, `Recent Runs`, `Connection Health`; không xếp 12 chart chỉ để tạo cảm giác nhiều tính năng.
- Empty: “Chưa có thiết bị — Kết nối Android để bắt đầu” + CTA tới Devices.

### 5.3 Devices — List + Inspector

- Data table columns: alias/serial masked, Android version, model, resolution, transport, status, assigned account, running workflow, last seen.
- Row click: side panel with device facts, screenshot on demand, action inspector, test buttons, logs. Không video streaming ở MVP.
- Bulk actions: refresh, group/tag, run workflow selection; destructive actions require per-device confirmation.
- Filter by USB/TCP, connected/disconnected, Android 9/10/11/12/13, busy/idle/error, assignment.
- Connection states: `disconnected`, `unauthorized`, `authorizing`, `ready`, `busy`, `recovering`, `blocked`.

### 5.4 Workflow Library

- Bảng/card chuyển chế độ có saved workflows, `name`, `version`, `nodes`, `last edited`, `last run`, `owner`, `validity`.
- CTA `Tạo quy trình`, `Import`, `Duplicate`, `Archive`; không xoá cứng ngay.
- Filters: game profile, type, tags, testing status, draft/published.
- Publish workflow phải qua validator và version snapshot; unsaved changes khuyến cáo trước điều hướng.

### 5.5 Workflow Studio — trang hero của sản phẩm

- Canvas ở trung tâm; **chỉ trong trang này** có Node Palette bên trái và Inspector bên phải.
- Graph toolbar nổi nhỏ trong canvas: zoom -/+, fit, grid, mini-map toggle, auto-layout, lock graph.
- Palette searchable, category accordions; drag node lên canvas hoặc click thêm sau node đang chọn.
- Custom nodes cấu trúc thống nhất: icon category, title, mini-config summary, input/output ports, status chip khi debug; node height 72–100px, min width 190–224px.
- Selected state: outline `stroke.focus`, không glow quá mạnh; error node: red small side indicator, tooltip lỗi.
- Edge: 2px default grey, accent blue on selected, green true, amber false/retry, muted red failure; keyboard highlight; edge label là vùng bấm được.
- Inspector tabs `Thiết lập`, `Đầu vào/Đầu ra`, `Lỗi & Retry`, `Ghi chú`; context-specific form dynamic theo schema node.
- Bottom Execution Dock tabs `Kết quả`, `Timeline`, `Logs`, `Ảnh xác minh`, `Biến`; desktop dock auto expands khi Run.
- Run options: dry-run/mock, run selected node, run from node (chỉ khi dependencies có data), run workflow. Rerun cảnh báo side effects.

**Không tái hiện trực tiếp lỗi của mẫu:** canvas không bị bóp giữa 3 cột cố định; side panels đều có collapse; log dock không luôn mở cao; trạng thái hệ thống chuyển xuống status bar.

### 5.6 Tasks / Task Set

- `Task Catalog`: bảng có toggle, task name, category, readiness condition, priority, estimate (nếu có data), version, state.
- `Selected Task Set`: danh sách reorderable có drag handle, priority override, `run once/loop within bounds`, end-policy.
- Bên phải: `Execution plan` dạng timeline sau khi scheduler đánh giá điều kiện, hiển thị skipped/waiting/ready lý do cụ thể.
- Quy tắc: nhiệm vụ người dùng tick là tập ứng viên; scheduler lấy **nhiệm vụ đủ điều kiện trước**, trong các job ready chọn priority rồi FIFO; không chạy hai tác vụ tương tác UI cùng lúc trên một device.
- End policy: `stop`, `idle`, `return_to_marker` (đòi capability + verify), `run_next_account` khi đã cấp quyền.

### 5.7 Runtime Monitor / Run Detail

- Header: trạng thái run, device, account alias, started/elapsed, cancel/pause.
- Node timeline: finished/failed/retry/running/waiting, bấm node xem input/output/screenshot.
- Log table virtualized, có filter, text search, copy redacted trace, export diagnostic bundle.
- Phân biệt `command_sent`, `device_ack`, `verification_passed`, `verification_failed`, `unknown`.
- Nếu mất kết nối giữa tap và verify: không tự lặp tap; đánh dấu `unknown` và xác minh lại state trước khi quyết định.

### 5.8 Accounts / Network / Settings

- Account list: alias, game profile, assigned device, state; tuyệt đối không hiện plaintext password/token; secrets lưu OS credential store.
- Network Profiles: loại Android system proxy / VPN app profile / external gateway, compatible/unsupported tags; test egress/DNS khi được phép.
- Account switch wizard: pause → checkpoint → logout/stop app → xóa app data chỉ nếu có quyền và đã xác nhận → apply network → verify → launch/login → verify account → resume task set của account mới.
- Settings: General, Devices, Security, Appearance, Logs & Storage, Backup, About; ở MVP không cần marketing/subscription màn hình giả.

---

## 6. Component System, UI Patterns & states

**Nền tảng:** Tailwind v4 utilities + shadcn/ui source-owned components (Radix accessibility), Lucide icons, các compound component của FarmerPro. **Không import cả 3 thư viện UI để lấy một button**; chọn một hệ primitives, custom token hóa.

### 6.1 Primitive tier

`Button` (primary/secondary/outline/ghost/destructive), `IconButton`, `Input`, `Textarea`, `Select`, `Checkbox`, `Switch`, `RadioGroup`, `Badge`, `Tabs`, `Tooltip`, `Popover`, `Dialog`, `AlertDialog`, `Sheet`, `DropdownMenu`, `Command`, `ContextMenu`, `Progress`, `Separator`, `Skeleton`, `Toast`, `ScrollArea`, `ResizablePanel`, `DataTable`.

### 6.2 FarmerPro compound tier

- `AppShell`, `NavRail`, `WorkspaceSwitcher`, `CommandBar`, `StatusBar`, `PageHeader`.
- `DeviceStatusChip`, `ConnectionBadge`, `DeviceRow`, `DeviceInspector`, `ScreenshotPreview`.
- `WorkflowCanvas`, `NodePalette`, `NodeCard`, `NodePort`, `WorkflowEdge`, `NodeInspector`, `CanvasToolbar`, `GraphMiniMap`.
- `TaskRow`, `TaskSetList`, `PrioritySelector`, `EligibilityIndicator`, `ExecutionPlan`, `RunTimeline`.
- `LogViewer`, `RunStateBadge`, `StepTraceDrawer`, `ErrorRecoveryPanel`, `CheckpointMarker`.
- `AccountSelect`, `AccountLeaseBadge`, `NetworkProfileSelect`, `SecretMaskedField`, `PermissionNotice`.

### 6.3 Component contracts

- Mọi control có 6 trạng thái: default, hover, focus-visible, active, disabled, invalid/loading nơi phù hợp.
- Button destructive không cùng sắc thái với primary. Action chạy thực tế có icon + label, không chỉ icon.
- Badge trạng thái dùng icon + text, không dựa độc màu. Error toast có mã lỗi và nút xem log.
- Loading skeleton chỉ render cho data đang chờ; không giả dữ liệu thành công khi chưa tải.
- Empty/error states có CTA hữu ích. Tooltip ở icon-only trong rail/toolbar.
- Tất cả dialog có Esc, focus trap; cả list có keyboard selection; `aria-label`, test accessibility.
- Accessibility desktop: mục tiêu WCAG 2.2 AA làm chuẩn thiết kế; test contrast và keyboard bằng axe + manual.

### 6.4 Form Node Inspector (chi tiết)

Một node có 5 phần: `Tóm tắt`, `Input`, `Action Parameters`, `Verification`, `Error Handling`. Chỉ mở từng nhóm khi cần.

**Random Safe Tap:**
- `Target source`: kết quả `Find Image`, `Find Text`, `UI Bounds`, hoặc vùng do người dùng chọn từ snapshot hiện tại.
- `Min confidence` (khi vision), `margin_ratio`, `min_region_px`, `max_tries`, `avoid_recent_points` (memory window), `max_move_age_ms`.
- `Device screen transform`: rotation/crop/insets từ frame metadata; không dùng pixel cũ nếu thiết bị đổi orientation.
- `Verification`: vùng ảnh kỳ vọng/trạng thái kỳ vọng, timeout, wait strategy.
- `on_failure`: fail, retry with recapture, branch, pause; không retry tap mù khi trạng thái chưa rõ.

### 6.5 UI states / wireframe contracts

- All views: `loading`, `ready`, `empty`, `no-permission`, `device-offline`, `backend-error`.
- Graph: `clean`, `dirty`, `validating`, `invalid`, `validated`, `running`, `paused`, `completed`, `failed`.
- Các màn hình phải có đủ mock data có dán **DEMO** và màn hình zero-state không data; tránh screenshot giả `24 devices` như dữ liệu thật.

---

## 7. Visual Design Skill — quy trình thiết kế bắt buộc

Đây là “skill protocol” áp dụng khi AI thiết kế FarmerPro, không phải một skill hệ thống đã cài đặt sẵn.

### Bước D0 — Evidence & Constraints

1. Đọc screenshot user, lập danh sách thuộc tính giữ/bỏ và bảng màu thực tế.
2. Chỉ ra 5 lỗi usability của layout cũ dựa trên ảnh; không đoán chức năng chưa có.
3. Xác định persona, task frequency, device count và độ phân giải; ghi design assumptions.

### Bước D1 — Information Architecture

1. Vẽ page map của 9 khu vực; chọn top 5 operator flows.
2. Lập shell hệ thống và quy tắc khi window bị resize.
3. Cắt bỏ duplicate actions, nav item không thuộc phase MVP.

### Bước D2 — Design Tokens + Components

1. Khóa tokens, spacing, typography, iconography, status semantics.
2. Thiết kế component gallery trước khi code business pages: từng component phải có default/hover/focus/disabled/error.
3. Không sử dụng gradient; motion ngắn và tắt được.

### Bước D3 — Screens & Flows

1. Thiết kế shell, Overview, Devices, Workflow Studio, Tasks, Runtime trước.
2. Phải xuất screenshot 1600×900, 1440×900, 1280×720; kiểm tra panel không lấn nhau.
3. Thực hiện click walkthrough: create node → configure → validate → run → inspect error → edit → rerun.

### Bước D4 — Strict Review

Cho mỗi screen chấm 10 điểm theo rubric: hierarchy 2, information density 1.5, workflow clarity 2, states 1, accessibility 1, visual consistency 1.5, window scaling 1. Không chấp nhận trung bình <9/10 hoặc lỗi critical về khả dụng; điểm là **review nội bộ**, không phải chứng nhận khách quan. Nếu <9, sửa và chụp lại; không lặp design cũ khi bị chê.

### Bước D5 — Evidence

Bàn giao token JSON, component showcase, 3 screenshot/window size mỗi screen, keyboard/focus evidence, storybook (hoặc equivalent), report pass/fail, và changelog. Không tuyên bố PASS nếu chưa chạy.

**Nếu tích hợp StarCi Harness:** trước khi dùng `@starci/grammar` hoặc `/claude-design`, đọc `contexts/README.md`, `contexts/` rules và `skills/claude-design/SKILL.md` tại phiên bản đã pin, kiểm tra tính tương thích với Vite/Tauri. Không mặc định StarCi Graph là Android workflow graph. StarCi là công cụ QA cho AI coding, không thay React Flow hoặc Rust Runtime.

---

## 8. Kiến trúc runtime Rust — modular, testable, local-first

```text
farmerpro/
├── README.md
├── AGENTS.md
├── LICENSE (chọn giấy phép phù hợp)
├── Cargo.toml                    # Rust workspace
├── rust-toolchain.toml           # pinned stable Rust
├── package.json / pnpm-lock.yaml # UI build only
├── apps/
│   ├── desktop/                  # Vite React TS app + Tauri shell
│   │   ├── src/
│   │   │   ├── app/               # providers/router/app shell
│   │   │   ├── design/            # tokens + gallery
│   │   │   ├── components/        # primitives + composites
│   │   │   ├── features/{devices,workflows,tasks,runs,accounts,network}/
│   │   │   ├── stores/            # editor state
│   │   │   └── lib/{ipc,schema,format}/
│   │   └── src-tauri/            # tauri v2, capabilities, commands
│   └── cli/                      # Rust CLI optional binary
├── crates/
│   ├── fp-domain/                # IDs, states, traits, domain errors
│   ├── fp-adb/                   # adb subprocess, USB/TCP, parsing
│   ├── fp-device/                # capabilities, geometry, actor, lease
│   ├── fp-vision/                # frame metadata, matching, OCR adapters
│   ├── fp-actions/               # tap/swipe/text/launch/verify, safe click policy
│   ├── fp-workflow-schema/       # versioned JSON contracts + validation
│   ├── fp-workflow-runtime/      # interpreter + durable checkpoints
│   ├── fp-scheduler/             # queues and device/account leases
│   ├── fp-storage/               # SQLite + migrations
│   ├── fp-security/              # secret references, permissions, audit
│   └── fp-testkit/               # deterministic fake device/screenshot fixtures
├── assets/{icons,templates,fixtures}/
├── docs/{architecture,design,decisions,acceptance}/
├── examples/workflows/
├── tests/{integration,e2e,visual}/
├── scripts/{build,verify,release}/
└── .github/workflows/{ci,release}.yml
```

**Không đặt business logic Rust vào Tauri commands.** `src-tauri` chỉ route typed commands/IPC và dependency injection vào services.

### 8.1 Rust Domain API (minh họa)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenGeometry {
    pub width_px: u32,
    pub height_px: u32,
    pub rotation_deg: u16,
    pub capture_ts_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionTarget {
    pub bounds: Bounds,
    pub confidence: f32,
    pub geometry: ScreenGeometry,
}

#[async_trait::async_trait]
pub trait DeviceDriver: Send + Sync {
    async fn snapshot(&self) -> anyhow::Result<Frame>;
    async fn geometry(&self) -> anyhow::Result<ScreenGeometry>;
    async fn tap(&self, x: u32, y: u32) -> anyhow::Result<ActionReceipt>;
    async fn swipe(&self, path: SwipePath) -> anyhow::Result<ActionReceipt>;
    async fn launch_app(&self, package: &str) -> anyhow::Result<ActionReceipt>;
}
```

`Frame`, `ActionReceipt`, `SwipePath` sẽ được định nghĩa ở `fp-domain`; đoạn trên là interface minh họa, **không phải file compile độc lập**.

### 8.2 ADB Driver

- Gọi `adb` qua `tokio::process::Command`, args list tường minh, không ghép shell string từ input người dùng.
- Dùng `adb devices -l`, `getprop`, `wm size`/`dumpsys` (fallback theo thiết bị), `exec-out screencap -p`, `shell input tap/swipe/keyevent`.
- Command có deadline, stdout/stderr limits, cancellation, structured errors, per-device serialization.
- Không tự động mở ADB TCP ra Internet. Nếu sử dụng TCP, yêu cầu trusted LAN/VPN, pair/authorize nếu hỗ trợ; không lưu key trần.
- Cài ADB qua Android SDK Platform Tools hợp lệ; nếu bundle `adb.exe`, phải review license/redistribution và chuẩn bị bản cập nhật riêng.

### 8.3 Vision Pipeline

- `FrameSource`: snapshot on demand, trả PNG + timestamp + device_geometry.
- `TargetDetector`: template match (OpenCV hoặc tương đương), OCR adapter (onnxruntime model), UI tree when available.
- `TargetNormalizer`: confidence threshold, non-max suppression khi có nhiều hits, size/mask, screen orientation/insets.
- `TargetVerifier`: nhận input state và điều kiện hậu action, phân biệt ảnh cũ/mới.
- `ModelRegistry`: artifact path, checksum, version, license, precision metrics, source.
- OCR không xử lý mọi frame: sự kiện workflow yêu cầu ảnh ở các bước verify/vision; bounded worker pool.
- Template assets lưu cùng game profile/version và resolution-independent descriptors nếu có.

### 8.4 Workflow Execution

- Workflow schema versioned, compile thành graph có kiểu, kiểm tra node/edge/ports, preconditions, bounded loops, deadline.
- Interpreter theo event-driven state machine (`Pending`, `Ready`, `Running`, `Waiting`, `Verifying`, `Succeeded`, `Failed`, `Paused`, `Unknown`, `Cancelled`).
- **Mỗi thiết bị một `DeviceActor` điều khiển thao tác UI có thứ tự**; toàn hệ thống có thể xử lý nhiều device actors đồng thời bằng Tokio tasks.
- `Run` được bind vào `workflow_version` bất biến; chỉnh draft không thay đổi run đang thực thi.
- Step có `timeout`, `retry` có `backoff`, `max_attempts`, `idempotency_class`, `verify`, `on_failure`; graph không được có vòng lặp không giới hạn.
- Checkpoint chỉ lưu sau verification có ý nghĩa; không coi ‘ADB command exit 0’ là game action thành công.
- Chuyển status `Unknown` nếu mất kết nối sau thao tác không idempotent; đọc lại trạng thái trước khi retry.

### 8.5 Scheduler & concurrency cho 20–50 máy

- Scheduler chịu trách nhiệm chọn jobs ready, device lease, account lease, token bucket cho Vision workers, fairness.
- Device lease `device_id` unique active; account lease `account_id` unique active; fencing generation để bỏ kết quả worker cũ.
- Job queue có priority và FIFO trong cùng priority; task availability dựa trên state+policy, không dựa vào màu badge GUI.
- CPU/OCR: bounded parallelism; chạy blocking compute qua `spawn_blocking` hoặc dedicated pool; không chạy OCR blocking ngay trên async executor.
- Hàng đợi screenshot có backpressure; bỏ screenshot stale, không bỏ checkpoint hoặc event audit.
- Không map 1 điện thoại = 1 physical thread; Tokio tasks có thể multiplex I/O.
- Thiết bị mất kết nối: lease giữ trong grace period có giới hạn, run paused/recovering; không đẩy job khác đè lên cho đến khi fencing/cleanup hoàn tất.

### 8.6 Workflow action / node registry

| Group | Node | Đầu vào | Đầu ra | Side-effect |
|---|---|---|---|---|
| Flow | Start/End | trigger | context | không |
| Device | Connect/Inspect | device id | capabilities | không/connection |
| App | Launch/Stop | package id | receipt | có |
| Vision | Capture | device | frame ref | không |
| Vision | Find Image/Find Text | frame+target | `VisionTarget` | không |
| Action | Random Safe Tap | target+policy | receipt/point | có, non-idempotent |
| Action | Swipe Region/Key | geometry/path | receipt | có |
| Logic | Wait/If/Else | condition/data | branch | không |
| Logic | Loop (bounded) | until/max iterations | iterations | tùy con |
| Verify | Wait For Target | target/timeout | evidence | không |
| Data | Set Variable | typed value | context change | không |
| State | Checkpoint | context | checkpoint id | có, idempotent |
| Task | Call Subworkflow | version+args | result | tùy con |
| Network | Authorized HTTP Request | allowlisted URL+schema | response | có; giới hạn quyền |

`NPC`, `Auto đánh quái`, `Tân thủ` là **task templates/subworkflows theo game**, không nên hard-code thành primitive node không thể sửa.

---

## 9. Không cố định tọa độ — Click Policy spec

### 9.1 Hệ tọa độ

**Không đánh đồng:**
- `capture_px`: pixel ảnh snapshot (có thể scale/crop/rotation).
- `display_px`: tọa độ input thực Android (ADB tap).
- `normalized_uv`: x/y trong [0,1] gắn với vùng màn hình.
- `world_xyz`: tọa độ nội bộ game; không được suy diễn từ screen coordinates.

Mọi detector trả `target bounds` + `frame metadata` + confidence. Trước khi click, validate freshness/capability và transform về `display_px` bằng matrix theo orientation/crop/insets. Nếu không xác định được transform: **fail closed**.

### 9.2 Chính sách Safe Random Tap

1. Nhận `bbox` hoặc `polygon mask` của nút tại frame mới.
2. Co vùng theo `margin_ratio` (mặc định thử nghiệm 0.18, cấu hình per target).
3. Giao với screen viewport, trừ mask cấm (overlay, thanh điều hướng, vùng nguy hiểm).
4. Nếu vùng còn lại < minimum area hoặc target confidence thấp → `TargetNotSafe`.
5. Lấy mẫu ngẫu nhiên trong polygon hợp lệ bằng RNG với seed chỉ dùng replay debug; giới hạn khoảng cách tối thiểu so với N điểm gần đây **khi diện tích đủ**.
6. Ghi vị trí đã chọn, target hash/capture version; thực thi một lần; chờ verify frame mới; nếu false, *recapture* chứ không spam tap.
7. Không biến `random tap` thành giải pháp tránh detection/anti-cheat; đây là chính sách bấm bên trong vùng UI hợp lệ, đảm bảo robustness khi kích cỡ thay đổi.

**Properties:** bounding box containment, normalized transform accuracy, target freshness, no banned mask intersections, finite retry budget, no spurious tapping when uncertainty.

### 9.3 Ví dụ pseudo-code

```text
safe_region = shrink(intersect(detected_bbox, screen_bounds), margin)
safe_region = subtract(safe_region, exclusion_masks)
if confidence < threshold or area(safe_region) < min_area: fail(TargetNotSafe)
point = sample_random(safe_region, avoid_recent_if_possible)
if !valid_transform(frame_geometry, current_device_geometry): fail(StaleGeometry)
point_device = transform(point, frame_to_device_matrix)
receipt = driver.tap(point_device)
result = verify_on_fresh_observation()
```

Lưu ý: `random` trong vùng **không đảm bảo** tránh lặp tọa độ tuyệt đối nếu vùng rất nhỏ hoặc lượng thao tác vô hạn; engine phải xử lý trường hợp không còn điểm đủ xa bằng retry/cảnh báo, không tạo tọa độ ngoài vùng.

---

## 10. Workflow Graph — dữ liệu và ngữ nghĩa thực thi

### 10.1 Graph contract

- `workflow_schema_version` bắt buộc (ví dụ `1`).
- `nodes[]`: id UUID, type, config validated, designer position (pure view metadata), `retryPolicy`, `timeout`, `onFailure`.
- `edges[]`: source node+port → target node+port; typed ports (`flow`, `frame`, `target`, `boolean`, `string`, `receipt`, `evidence`).
- Graph xác nhận có đúng Start, ít nhất End reachable, mọi node reachable hoặc cảnh báo unused; không có implicit loops ngoài `Loop`/`RepeatUntil` có giới hạn.
- Editor có draft graph riêng; Runtime chỉ nhận **versioned/published snapshot** và `device_run_context`.
- `node_id` ổn định qua edit; đổi tên không thay ID; diff được version.

### 10.2 Workflow mẫu “Nhiệm vụ tân thủ”

```text
Start
 → Inspect Device
 → Launch App
 → Wait For Home
 → Find Quest Marker
 → Random Safe Tap (marker bounds)
 → Wait Dialogue
 → IF Quest Available?
    ├─ YES → Tap Accept → Verify Quest Active → Checkpoint → End
    └─ NO  → Recovery Dialog → Retry (max 3) → Pause Manual Review
```

`Auto đánh quái` là subworkflow game-specific: detect target → verify valid state → control interaction → observe result → repeat until bounded condition. Không assume OCR phát hiện đúng mọi frame hoặc server xác nhận mọi hành động.

### 10.3 Workflow example JSON

Xem tệp `FARMERPRO_SAMPLE_WORKFLOW.json` trong bộ bàn giao: dữ liệu minh họa có thể validate bằng schema riêng khi đã triển khai; chưa phải input chạy được trên phần mềm đã xây dựng.

### 10.4 Execution semantics

- Graph runtime nhận event `NodeReady`; các node điều khiển cùng device nằm trong serialized action channel.
- IF/ELSE chỉ một nhánh được mở khi predicate được xác nhận; `Unknown` không tự coi là false.
- Loop có `maxIterations`, `maxDuration`, `until` và per-iteration checkpoint option.
- Pause là cooperative: không ngắt giữa subprocess/native action thiếu hỗ trợ cancellation; Stop là graceful, emergency stop hủy pending và khóa input.
- Manual override phải yêu cầu lease takeover; khi người dùng điều khiển thiết bị, automation tạm dừng và xác minh lại trạng thái khi resume.

---

## 11. Dữ liệu, persistence, audit

### 11.1 SQLite schema đề xuất

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS devices (
  id TEXT PRIMARY KEY,
  adb_serial TEXT UNIQUE,
  alias TEXT NOT NULL,
  android_api INTEGER,
  model TEXT,
  width_px INTEGER,
  height_px INTEGER,
  rotation_deg INTEGER,
  connection_state TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflows (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  draft_json TEXT NOT NULL,
  draft_updated_at TEXT NOT NULL,
  archived_at TEXT
);

CREATE TABLE IF NOT EXISTS workflow_versions (
  id TEXT PRIMARY KEY,
  workflow_id TEXT NOT NULL REFERENCES workflows(id),
  version_no INTEGER NOT NULL,
  graph_json TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(workflow_id, version_no)
);

CREATE TABLE IF NOT EXISTS accounts (
  id TEXT PRIMARY KEY,
  alias TEXT NOT NULL,
  game_profile_id TEXT,
  secret_ref TEXT,
  state TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES devices(id),
  account_id TEXT REFERENCES accounts(id),
  workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(id),
  priority INTEGER NOT NULL DEFAULT 0,
  state TEXT NOT NULL,
  fencing_generation INTEGER NOT NULL DEFAULT 0,
  current_node_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS step_events (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL REFERENCES jobs(id),
  node_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  attempt_no INTEGER NOT NULL,
  at TEXT NOT NULL,
  payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS checkpoints (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL REFERENCES jobs(id),
  node_id TEXT NOT NULL,
  state_json TEXT NOT NULL,
  evidence_ref TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS device_leases (
  device_id TEXT PRIMARY KEY REFERENCES devices(id),
  job_id TEXT NOT NULL REFERENCES jobs(id),
  generation INTEGER NOT NULL,
  expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS account_leases (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id),
  job_id TEXT NOT NULL REFERENCES jobs(id),
  generation INTEGER NOT NULL,
  expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_state_priority ON jobs(state, priority DESC, created_at);
CREATE INDEX IF NOT EXISTS idx_events_job_time ON step_events(job_id, at);
```

Các bảng bổ sung ở P3+: `network_profiles`, `task_sets`, `task_definitions`, `task_runs`, `asset_templates`, `audit_logs`, `screen_captures`, `settings`, `release_metadata`. Dùng migrations có version, không tạo schema bằng string trong runtime sản phẩm.

### 11.2 Privacy & retention

- Mặc định screenshot debug chỉ lưu khi explicitly enabled hoặc khi lỗi (người dùng chọn retention); có redaction cho account/user data.
- Logs chứa event structured, không log password/token/proxy auth; giới hạn disk quota và retention.
- SQLite ở Tauri `app_data_dir`, không ở Program Files; backup/export phải có opt-in và dữ liệu bí mật được mã hóa/loại bỏ.
- Secrets tham chiếu từ Windows Credential Manager / OS keyring, không nhét JSON workflow.

---

## 12. Tauri IPC, API và event contracts

**Local-first:** Renderer React không kết nối trực tiếp ADB hay đọc filesystem tùy ý. Tauri commands có xác thực origin/capabilities và call into Rust services.

### 12.1 Commands mẫu

```text
app_get_status()                         -> AppStatus
list_devices()                           -> Vec<DeviceSummary>
inspect_device(device_id)                -> DeviceDetail
capture_device(device_id)                -> CaptureRef
create_workflow(draft)                   -> WorkflowId
save_workflow_draft(workflow_id,draft)   -> SaveResult
validate_workflow(workflow_id)           -> ValidationReport
publish_workflow(workflow_id)            -> WorkflowVersion
run_workflow(workflow_version_id,device_id,account_id?) -> JobId
pause_job(job_id)                        -> JobState
resume_job(job_id)                       -> JobState
stop_job(job_id,reason)                  -> JobState
get_run_timeline(job_id)                 -> Vec<StepEvent>
execute_test_node(device_id,node_config) -> NodeTestResult
```

### 12.2 Events mẫu

```text
fp://device-status
fp://job-status
fp://node-started
fp://node-result
fp://job-log
fp://connection-alert
```

Payload có `event_id`, `job_id`, `device_id`, `timestamp`, `schema_version`; frontend phải xử lý reconnect/snapshot reconciliation, không tin rằng mọi event sẽ đến đúng thứ tự hoặc chỉ đến một lần.

### 12.3 HTTP API tương lai

Nếu phát triển WebApp từ xa, tạo `Axum` server riêng + authentication, CSRF/CORS/authorization, TLS, WebSocket/SSE; không mở Tauri IPC trực tiếp lên mạng. MVP không cần server riêng nếu Tauri IPC đủ.

### 12.4 Network/API nodes

HTTP Request node chỉ dùng API hợp lệ/được ủy quyền, allowlist/SSRF protections, redaction, timeout, credential references. **Không hứa request trực tiếp thay thế UI thao tác game nếu chưa có giao thức được công bố/cho phép.**

---

## 13. Accounts, Network Profiles và các thao tác rủi ro

- `AccountProfile`: id/alias/game profile/secret ref/last verified identity, immutable run binding.
- `NetworkProfile`: type, user-authorized config, health test, application applicability, status (supported, limited, unsupported).
- **Đổi proxy phía máy tính không tự làm đổi mạng game trên Android.** Android system proxy có thể bị app bỏ qua; VPN cần quyền và chính sách Android.
- `Clear app data` qua ADB có thể xóa dữ liệu game và cần confirmation; APK Agent không mặc định được quyền xóa app khác. Thiết kế fallback qua logout chính thức/manual review.
- Account switch không reuse quest checkpoint cho một account khác; tài khoản mới phải đọc lại state/tập nhiệm vụ đã chọn.
- Không ghi hoặc xuất hiện plaintext password/token trong GUI/screenshot/log; audit ai/thiết bị/nhiệm vụ/thời gian/loại thay đổi.
- Mỗi hành động destructive có dry-run summary và nút xác nhận; không chạy silent account switch khi thông tin quyền chưa rõ.

---

## 14. Test strategy và tiêu chí chất lượng

### 14.1 Test pyramid

- **Unit Rust:** graph validation, retry policies, coordinate transforms, safe region sampler, lease/fencing, priority sorting, timeout/cancel, serialization.
- **Property-based Rust:** bboxes bất kỳ, rotations 0/90/180/270, hình chữ nhật nhỏ/lớn, exclusion polygons, random points always inside, no divide-by-zero/overflow.
- **Golden screenshot tests:** templates từ nhiều màn hình/brands, kiểm tra false positives và target confidence; dataset labels phải rõ.
- **Integration ADB fake:** fake DeviceDriver trả frame có kiểm soát; test timeout/recovery/idempotency unknown.
- **Real Android:** Android 9, 11, 13, tối thiểu 2 hãng/3 độ phân giải, portrait/landscape và thao tác UI test app có quyền.
- **Frontend component & e2e:** Vitest, Testing Library, Playwright/Tauri WebDriver nếu phù hợp; interaction graph editing, keyboard accessibility, corrupted/import invalid workflow.
- **Visual regression:** 1600×900, 1440×900, 1280×720; screenshot baseline được chấp thuận, diff threshold có giải trình.
- **Packaging:** clean VM Windows 11, install, first-run, connect adb, execute fake workflow, uninstall, verify user data policy.
- **Scale:** 5 → 20 → 50 devices: CPU, RSS, disk I/O, screenshot throughput, event queue backlog, leaked leases, crash recovery.

### 14.2 Gate classes

| Gate | Phải đạt trước khi merge |
|---|---|
| Rust | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, security review cargo-audit khi tích hợp |
| UI | `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build` |
| Graph | schema tests, invalid edges, bounded loop tests, migration test |
| Design | component gallery, keyboard, contrast, 3 viewport screenshots, không overflow |
| E2E | tạo workflow, lưu/mở, validate, dry-run, step-run, pause/resume |
| Installer | `tauri build`, chạy installer trên Windows clean VM, SHA256, SBOM/provenance theo release policy |

### 14.3 Không được đánh dấu PASS nếu

- chỉ có mock UI, không có adb action thật nhưng tuyên bố đã điều khiển máy;
- chỉ trả status success vì command exit 0, chưa verify ảnh/game state;
- random tap dùng cứng một tọa độ, bỏ qua orientation hoặc vùng cấm;
- scheduler chạy 2 workflow điều khiển cùng một device;
- chạy `cargo build` nhưng chưa kiểm tra app.exe và Windows installer ở đúng OS;
- screenshots mới chỉ là mock, chưa chụp từ app build.

---

## 15. Packaging Windows — EXE rõ ràng, không đánh đồng khái niệm

### 15.1 Tauri runtime trên Windows

- Tauri 2 sử dụng **Microsoft Edge WebView2** để render React/Vite UI. Tệp `.exe` chương trình không tương đương “zero dependency”: runtime WebView2 phải tồn tại hoặc được installer cài/đóng gói theo chính sách.
- Người dùng cuối **không cần Node.js hoặc Python** để chạy bản release, nếu không dùng Python sidecar và các thư viện native đã được gói đúng. Build frontend yêu cầu Node và package manager.
- Tauri có thể phát hành **NSIS setup executable** `*-setup.exe` và **MSI**; chọn NSIS là định dạng phân phối chính, MSI là tùy chọn deployment managed.
- Nếu cần **portable single EXE**: đây là target *riêng*, không tự xuất hiện chỉ vì `tauri build`; vẫn phải xét WebView2, DLL, asset sidecar, ADB và các quyền truy cập.

### 15.2 Yêu cầu môi trường build (Windows)

- Windows 11 x64; Rustup stable MSVC target; Visual Studio Build Tools/C++ workload; Edge WebView2 runtime; Node phù hợp LTS/được project pin; pnpm qua Corepack (hoặc package manager đã chọn).
- Tauri 2 CLI và `tauri.conf.json` theo docs phiên bản đã pin; không copy config cũ Tauri 1.
- Android SDK Platform Tools phải được tìm thấy rõ (user path hoặc app-owned path có license/updates được giải quyết).

### 15.3 Ví dụ câu lệnh xây dựng (cần điều chỉnh theo package scripts thực tế)

```powershell
# Môi trường và công cụ
rustc --version
cargo --version
node --version
pnpm --version

# Build frontend
pnpm --filter @farmerpro/desktop install
pnpm --filter @farmerpro/desktop build

# Tauri production build (từ đúng thư mục app hoặc qua script workspace)
pnpm --filter @farmerpro/desktop tauri build

# Hoặc gọi cấu hình scripts được định nghĩa của project
pnpm desktop:build
```

Không coi đây là lệnh có thể chạy ngay trước khi tạo repo; đó là **target commands** trong blueprint.

### 15.4 Artifact release

```text
release/
  FarmerPro_1.0.0_x64-setup.exe      # NSIS installer, mục tiêu chính
  FarmerPro_1.0.0_x64.msi            # optional
  SHA256SUMS.txt
  release-notes.md
  SBOM.json                           # optional/production
  signed-build-manifest.json          # nếu ký số
```

**Code-signing** cần xem xét khi phát hành rộng rãi. EXE chưa ký có thể bị Windows SmartScreen cảnh báo; ký số và tích lũy reputation giảm rào cản nhưng không bảo đảm 100% tránh cảnh báo.

### 15.5 Installer UX

- Per-user install mặc định, không yêu cầu admin nếu không cần; shortcuts tùy chọn.
- First run wizard kiểm tra ADB SDK/USB debugging/authorization.
- Uninstall không tự xóa user workflows/data nếu không có opt-in rõ; có tùy chọn `Remove all local data` ở cuối.
- Crash report opt-in, không tự upload ảnh chứa thông tin tài khoản.

---

## 16. StarCi Harness — cách dùng đúng vai trò

**Đã xác minh từ repository:** `tcandt/starci-harness` là fork của `starci183/starci-harness` (MIT), CLI Node 24+, có task contracts, clean verification, SQLite ledgers, `@starci/grammar`, Tauri app. Repo phát triển AI agents chứ không điều khiển điện thoại Android.

**Ứng dụng vào FarmerPro:**

1. Đọc README, `contexts/README.md`, `skills/starci/SKILL.md`, `skills/claude-design/SKILL.md`, `AGENTS.md`, `policy/`, xác nhận API và quyền thực thi.
2. Cài `starci` trong môi trường dev riêng (không bundle vào EXE FarmerPro), pin version.
3. Tạo FarmerPro repository riêng; có `AGENTS.md` + `.starciwork/` nếu phiên bản harness yêu cầu.
4. Chia các deliverable thành contract có allowPaths, checks, định nghĩa DONE; build/verify trên clean checkout.
5. Nếu dùng `@starci/grammar` để tham khảo, chỉ đưa vào UI sau khi đánh giá tương thích; mặc định ưu tiên **shadcn/ui + FarmerPro tokens** để tránh hai hệ primitives xung đột.
6. **Không tự cho agent quyền commit/push/publish vượt phạm vi**. Chỉ làm sau khi chủ sở hữu phê duyệt theo chính sách harness; phát hành package/StarCi khác với phát hành FarmerPro.

**Không làm:** fork StarCi rồi cố biến workflow AI thành hệ thống Android; sửa thẳng repo StarCi cho FarmerPro khi chưa có mục đích cụ thể; cài StarCi global làm dependency bắt buộc của người dùng cuối.

---

## 17. Lộ trình sản phẩm: mốc, đầu ra, nghiệm thu

### PHASE 0 — Discovery & Design (M0)

**Work:** kiểm tra ảnh/giao diện cũ, D0–D4 Design Skill, chọn tokens, hoàn thiện IA, wireframes 4 màn quan trọng, component gallery.

**Đầu ra:** design tokens, screenshot gallery 1600×900/1440×900/1280×720, component inventory, UX acceptance. **Gate:** review ≥ 9/10 nội bộ, không lỗi overflow/interaction critical.

### PHASE 1 — Bootstrap + EXE Proof (M1)

**Work:** mono-repo, Rust workspace, Tauri shell, React Vite TS, Tailwind v4, shadcn/ui, React Flow, SQLite init, installer build.

**Đầu ra:** FarmerPro opens on Windows, shell/sidebar/header/canvas empty, hello Rust IPC, NSIS setup exe. **Gate:** cài/chạy/gỡ trên Windows test VM, no Node installed on target VM.

### PHASE 2 — Workflow Builder UI (M2)

**Work:** custom nodes, palette, inspect, typed ports, edges, graph validate, save/load/import/export versioned drafts, undo/redo.

**Đầu ra:** 10–15 node GUI, tạo pipeline `Start→Find Image→Random Tap→Verify→End`, no mock pretending runtime. **Gate:** edit interactions/keyboard/viewport screenshots.

### PHASE 3 — ADB Rust & Vision (M3)

**Work:** device discovery, screenshot-on-demand, process isolation, screen transform, template image detect, safe region random sampler, actions.

**Đầu ra:** chạy ADB action với device test, ảnh evidence trước/sau, logs. **Gate:** 3 thiết bị, 3 kích cỡ, landscape, error handling.

### PHASE 4 — Runtime + Debugger (M4)

**Work:** executor, typed context, step verifier, pause/resume, bounded retry, checkpoint, runtime inspector, safe recovery.

**Đầu ra:** workflow end-to-end chạy trên 1 thiết bị và test app với ảnh mục tiêu; UI hiển thị real trace. **Gate:** 100 test runs, no silent wrong success.

### PHASE 5 — Tasks + Multi-device (M5)

**Work:** task library, selected task set, scheduling/priority, device/account lease, concurrency caps, 5→20→50 stability.

**Đầu ra:** task sequencing, job queue, bulk actions, device overview and diagnostics. **Gate:** 20+ test devices hoặc simulated load được ghi rõ; để tuyên bố hỗ trợ 50 thực tế cần 50 real devices/lab equivalent.

### PHASE 6 — Account/Proxy & Production Polish (M6)

**Work:** credential store, network profiles, cautious account switching, audit, update signing, installer QA, docs.

**Đầu ra:** release candidate signed nếu có chứng thư, user manual, backup, recovery plan. **Gate:** permissions/secret handling, Windows VM smoke, regression, no destructive default.

### PHASE 7 — Cloud/Agent/Streaming (để sau)

Chỉ lập RFC sau khi M6 ổn định và đo nhu cầu. Không đưa vào MVP hoặc báo cáo đã hoàn thành.

---

## 18. Release acceptance: bắt buộc trước khi gọi FarmerPro “production ready”

### 18.1 UI/UX

- [ ] Shell hoàn toàn mới, giữ dark navy/blue; không lặp layout mẫu đã bị phàn nàn.
- [ ] Sidebar collapsed/expanded, keyboard nav, active nav rõ ràng.
- [ ] Header responsive và buttons có trạng thái hợp lý.
- [ ] 3 kích thước cửa sổ không overlap, clipping, horizontal scroll không mong muốn.
- [ ] Canvas pan/zoom/fit, custom node/port/edge, retry conditional branch, labels đúng.
- [ ] Inspector form validation/error help, bottom logs collapsible, demo marker cho mock.
- [ ] Không gradient nền, shadow quá đậm hoặc 10 loại badge màu không quy tắc.

### 18.2 Workflow/Device

- [ ] Kết nối ADB Android 9–13 ít nhất 3 cấu hình; mất kết nối được phát hiện.
- [ ] Snapshot + metadata chuẩn, orientation transform được kiểm thử.
- [ ] Template/OCR tìm mục tiêu trả bbox/confidence; khi không tìm thấy không tap.
- [ ] Random tap luôn thuộc safe region và loại vùng cấm; không dùng một tọa độ hard-coded chung.
- [ ] Wait/If/Else/Loop/Retry/Checkpoint hoạt động và bounded.
- [ ] Thực thi một workflow thật có ảnh trước/sau và verified result.
- [ ] Device actor chỉ có một stream thao tác; account không chạy trùng job.
- [ ] Pause/Stop/Manual override không gây replay lệnh nguy hiểm.

### 18.3 Production

- [ ] Schema migration/upgrades, backup/export, logs redacted, secrets OS store.
- [ ] CI gates Rust + frontend + e2e + visual passing.
- [ ] Có `FarmerPro.exe` native binary và **NSIS setup `.exe`** chạy trên Windows test máy sạch.
- [ ] Cài/first-run/connect/test/uninstall hoàn thành; release SHA256 and docs.
- [ ] Báo cáo ngưỡng tải 5/20/50 phân biệt simulated vs real hardware.
- [ ] Không tuyên bố cloud streaming nếu chưa xây dựng.

---

## 19. Anti-patterns tuyệt đối tránh

1. Thiết kế UI nặng màu, 3 sidebars vĩnh viễn, lấp canvas bằng panels.
2. Ghép code frontend trực tiếp điều khiển adb hoặc mở command shell tùy ý.
3. Hard-code resolution/`tap(500,800)` và gọi nó là đa thiết bị.
4. Giữ `confidence` của screenshot cũ để click trên frame mới khi không xác minh state.
5. Retry `tap` sau disconnect khi chưa biết lệnh đã được game thực thi hay chưa.
6. Tạo 50 native threads bắt buộc chỉ vì có 50 điện thoại.
7. Dùng Node.js/Electron cho production runtime khi mục tiêu là Rust/Tauri.
8. Gán `npm install` là đã hoàn tất build Windows EXE.
9. Mặc định xóa app data, bypass login, can thiệp API nội bộ game không có quyền.
10. Nhầm StarCi Harness với runtime workflow cho Android.
11. Khẳng định 10/10 UI mà chưa screenshot/rubric/acceptance.
12. Push Git, release hay publish chưa có quyền/chưa xác minh.

---

## 20. HƯỚNG DẪN DÙNG MASTER PROMPT

Tệp **`FARMERPRO_MASTER_IMPLEMENTATION_PROMPT.md`** đi kèm là prompt “đưa thẳng cho AI coding agent”; tài liệu này là nguồn sự thật của yêu cầu sản phẩm. Agent phải đọc blueprint trước, không viết toàn bộ sản phẩm trong một lần mà không kiểm thử, và phải cho thấy bằng chứng thật.

**Thứ tự nên giao AI:** Design system → Tauri installer proof → Graph Builder → Rust ADB → Vision & Random Safe Tap → Workflow runtime → Multi-device → Accounts/Proxy → Production.

### Các đầu ra cần xuất theo mỗi Phase

```text
phase-report.md
- Scope/commits/changed files
- Commands actually executed and exit codes
- Screenshots at 1600/1440/1280 where relevant
- Tests PASS/FAIL with environment
- Known limitations + next phase dependencies
- Windows artifact path and sha256 if building
- No unverified "completed" claims
```

---

## 21. Nguồn tài liệu kỹ thuật (đã kiểm tra ngày 20/11/2026)

1. Tauri v2 Windows installer: https://v2.tauri.app/distribute/windows-installer/
2. Tauri prerequisites/WebView2: https://v2.tauri.app/start/prerequisites/
3. React Flow custom nodes: https://reactflow.dev/examples/nodes/custom-node
4. React Flow handles/validation: https://reactflow.dev/learn/customization/handles
5. React Flow v13 release (16/11/2026): https://reactflow.dev/whats-new/2026-11-16
6. shadcn/ui Vite: https://ui.shadcn.com/docs/installation/vite
7. Microsoft SmartScreen reputation: https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation
8. StarCi harness (user fork): https://github.com/tcandt/starci-harness
9. StarCi desktop app README: https://github.com/tcandt/starci-harness/blob/main/app/README.md
10. StarCi skill (explicit opt-in): https://github.com/tcandt/starci-harness/blob/main/skills/starci/SKILL.md

**Cần xác minh khi triển khai:** phiên bản cụ thể của Rust/Tauri/React Flow/Tailwind, package APIs và mọi giới hạn thực tế trên model Android/game. Không khẳng định từ blueprint là đã chạy được.

---

**HẾT BLUEPRINT V1.0 — FarmerPro | Rust-first | Windows EXE | Workflow Studio | No Video Streaming MVP**
