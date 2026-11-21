import os

DIAGRAM_DIR = r"E:\DATA\FarmerPro\artifacts\p1_2_evidence\diagrams"
os.makedirs(DIAGRAM_DIR, exist_ok=True)

# 1. Save editable Mermaid (.mmd) source files for all architecture diagrams
MERMAID_SOURCES = {
    "00_part1_overall_product_architecture.mmd": """flowchart LR
    subgraph STUDIO["FarmerPro Studio IDE (FarmerPro.exe)"]
        CTX["Global Context Header: Project Switcher + Active Lab Device"]
        W1["1. Game Projects / Workspace"]
        W2["2. Workflow Studio (React Flow + Vision/OCR)"]
        W3["3. Tasks & Modules (Library, Vault, Proxy)"]
        W4["4. Device Lab (ADB, LDPlayer, ROI Cropper)"]
        W5["5. Test & Debug (Timeline, Before/After, Checkpoints)"]
        W6["6. Mini Tool Builder & Releases"]
    end
    subgraph ENGINE["Shared Rust Core Workspace (14 Crates)"]
        CORE["fp-domain · fp-adb · fp-device · fp-vision · fp-actions\\nfp-workflow-schema · fp-workflow-runtime · fp-scheduler\\nfp-storage (SQLite WAL) · fp-security"]
        CONTRACT["fp-bundle-contract + fp-bundle-verify"]
        BUILDER["fp-builder (8-Step Packager + PE RT_RCDATA #7777 + Ed25519 Signer)"]
    end
    subgraph OUTPUT["Standalone Mini Auto-Tools (Zero Studio Dependency)"]
        M1["Auto_JX2_Mobile.exe + signed bundle/ + data/runtime_jx2-mobile.db"]
        M2["Auto_Kingdom_SLG.exe + signed bundle/ + data/runtime_kingdom-slg.db"]
    end
    STUDIO --> CORE
    W6 --> BUILDER
    BUILDER --> CONTRACT
    BUILDER ==> M1
    BUILDER ==> M2
""",
    "00_part2_before_vs_proposed_architecture.mmd": """flowchart TB
    subgraph BEFORE["KIẾN TRÚC CŨ (v0.1.0 - 10 Tab Vận Hành Phẳng)"]
        B1["01. Tổng quan (Vận hành đơn lẻ)"]
        B2["02. Thiết bị Android"]
        B3["03. Kho Quy trình JX2 (Hardcode)"]
        B4["04. Workflow Studio"]
        B5["05. Bộ Nhiệm vụ (Tách rời)"]
        B6["06. Phiên chạy (Có dữ liệu demo)"]
        B7["07. Tài khoản Vault (Phân tán)"]
        B8["08. Mạng & Proxy (Phân tán)"]
        B9["09. Design Tokens"]
        B10["10. Cài đặt & ADB"]
    end
    subgraph AFTER["KIẾN TRÚC ĐỀ XUẤT P1.2 (Studio IDE Đa Dự Án - 6 Khu Vực + Hệ Thống)"]
        A1["1. Game Projects / Workspace (Multi-Game Hub, Assets, Health)"]
        A2["2. Workflow Studio (Project Explorer, Canvas, Inspector, Execution Dock)"]
        A3["3. Tasks & Modules (Task Library, Categories, Account Vault, Proxy, Watchdog)"]
        A4["4. Device Lab (ADB, LDPlayer, Live Snapshot, ROI Template Cropper)"]
        A5["5. Test & Debug (Real SQLite Traces, Before/After Shots, Checkpoints)"]
        A6["6. Mini Tool Builder & Releases (Form Designer, Live Preview, 8-Step Build EXE)"]
        ASYS["System Settings Drawer (ADB Path, Producer Ed25519 Keystore, SQLite Backup, Design Tokens)"]
    end
    B1 --> A1
    B2 --> A4
    B3 --> A1
    B3 --> A2
    B4 --> A2
    B5 --> A3
    B6 --> A5
    B7 --> A3
    B8 --> A3
    B9 --> ASYS
    B10 --> ASYS
""",
    "IV_01_cargo_workspace_dependency_graph.mmd": """flowchart TB
    subgraph APPS["Application Binaries (apps/*)"]
        DESKTOP["apps/desktop (FarmerPro.exe Studio IDE)"]
        CLI["apps/cli (fp-cli.exe Diagnostic & Builder CLI)"]
        MINI["apps/mini-runtime (fp-mini-runtime.exe -> Auto_<Game>.exe)"]
    end
    subgraph BUILD_LAYER["Producer Packaging Layer (Studio/CLI Only)"]
        FP_BUILDER["crates/fp-builder (8-Step Packager, PE Icon & RT_RCDATA #7777 Injector, Ed25519 Signer)"]
    end
    subgraph BUNDLE_LAYER["Bundle Contract & Verification Layer"]
        FP_VERIFY["crates/fp-bundle-verify (PE RT_RCDATA #7777 Reader, RFC 8032 Verifier, Path Guard)"]
        FP_CONTRACT["crates/fp-bundle-contract (Zero-Crypto Schemas, 5-State Module Matrix, Dependency Audit)"]
    end
    subgraph RUNTIME_CORE["Shared Automation Core Crates (Unchanged Across Games)"]
        FP_WF_RT["crates/fp-workflow-runtime"]
        FP_WF_SC["crates/fp-workflow-schema"]
        FP_SCHED["crates/fp-scheduler"]
        FP_DEV["crates/fp-device"]
        FP_ADB["crates/fp-adb"]
        FP_VIS["crates/fp-vision"]
        FP_ACT["crates/fp-actions"]
        FP_STOR["crates/fp-storage"]
        FP_SEC["crates/fp-security"]
        FP_DOM["crates/fp-domain"]
    end
    DESKTOP --> FP_BUILDER
    CLI --> FP_BUILDER
    FP_BUILDER --> FP_VERIFY
    FP_BUILDER --> FP_CONTRACT
    MINI --> FP_VERIFY
    MINI --> FP_CONTRACT
    MINI -. "NEVER DEPENDS ON (Compile-Time Isolation)" .-> FP_BUILDER
    DESKTOP --> RUNTIME_CORE
    MINI --> RUNTIME_CORE
""",
    "IV_02_farmerpro_studio_architecture.mmd": """flowchart LR
    subgraph UI["Frontend: Tauri v2 WebView + React 19 + TypeScript + Tailwind v4"]
        HDR["Global Context Bar (Active Project + Active Lab Device)"]
        WS["6 Studio Workspaces (Projects, Workflow Studio, Tasks/Modules, Device Lab, Test/Debug, Mini Builder)"]
        RF["React Flow Graph Canvas + CustomWorkflowNode + Auto-Layout"]
        ZUS["Zustand Studio Store (Multi-Project Context & Reactive State)"]
    end
    subgraph IPC["Tauri v2 Typed IPC Commands"]
        CMD["studio_list_projects · studio_crop_and_save_template\\nstudio_upsert_project_task · studio_save_ui_schema\\nstudio_preflight_check_release · studio_build_mini_tool_release"]
    end
    subgraph BACKEND["Rust Studio Backend (src-tauri)"]
        STOR["fp-storage (SQLite WAL farmerpro.db v2 Multi-Project)"]
        DEVLAB["fp-adb + fp-device + fp-vision (Live Snapshot & ROI Cropper)"]
        WFRT["fp-workflow-runtime + fp-scheduler (Step-Run & Trace Engine)"]
        BLD["fp-builder + ProducerKeyManager (8-Step EXE Packager)"]
    end
    HDR --> ZUS
    WS --> ZUS
    RF --> ZUS
    ZUS <--> CMD
    CMD --> STOR
    CMD --> DEVLAB
    CMD --> WFRT
    CMD --> BLD
""",
    "IV_03_gameproject_data_model.mmd": """erDiagram
    PROJECTS ||--o{ PROJECT_MODULES : configures
    PROJECTS ||--o{ VISION_TEMPLATES : owns
    PROJECTS ||--o{ WORKFLOWS : contains
    PROJECTS ||--o{ TASK_DEFINITIONS : publishes
    PROJECTS ||--|| MINI_TOOL_SCHEMAS : defines_ui
    PROJECTS ||--o{ PROJECT_RELEASES : builds
    PROJECTS ||--o{ ACCOUNTS : scopes_vault
    PROJECTS ||--o{ JOBS : records_runs
    WORKFLOWS ||--o{ TASK_DEFINITIONS : mapped_to
    JOBS ||--o{ STEP_EVENTS : emits
    JOBS ||--o{ CHECKPOINTS : saves
""",
    "IV_04_workflow_runtime_execution_architecture.mmd": """flowchart LR
    START(["flow.start"]) --> CAP["vision.capture\\n(AdbRunner screencap -p PNG)"]
    CAP --> FIND["vision.find_image\\n(Multiscale SSD Template Match)"]
    FIND -->|Matched >= minConfidence| SAFE["action.random_safe_tap\\n(18% Margin Shrink + Exclusion Masks)"]
    FIND -->|Below Threshold| RETRY{"Retry Policy\\n(Attempt <= Max?)"}
    RETRY -->|Yes + Backoff| CAP
    RETRY -->|Exhausted| FAIL(["JobState::Failed\\n(Emit step_failed event)"])
    SAFE --> TAP["AdbRunner: shell input tap X Y\\n(Explicit Args Vector)"]
    TAP --> WAIT["flow.wait / Post-Condition"]
    WAIT --> OCR["vision.ocr\\n(WinRT OcrEngine Extract Text/Vars)"]
    OCR --> CP["state.checkpoint\\n(Atomic SQLite WAL Persist)"]
    CP --> END(["flow.end (Succeeded)"])
""",
    "IV_05_adb_emulator_adapter_architecture.mmd": """flowchart TB
    subgraph HOST["Rust Host Process (No Raw Shell Interpolation)"]
        RUNNER["fp-adb::AdbRunner\\ntokio::process::Command::new(adb_path).args(&[...])"]
        DISC["Device Discovery & LAN TCP Scanner\\n(USB Serials + 127.0.0.1:5555..5585 + LAN /24)"]
        EMU["Emulator Adapter Controller\\n(LDPlayer ldconsole.exe / Nox / BlueStacks)"]
        ACTOR["fp-device::DeviceActor\\n(Per-Device Mailbox + Generation Fencing Token)"]
        GEOM["Geometry & Orientation Normalizer\\n(wm size 1280x720 + Rotation 0/90/180/270 Transform)"]
    end
    subgraph TARGETS["Connected Android Targets"]
        LD["LDPlayer Instance (emulator-5554 / SM_X710N)"]
        TCP["TCP/IP Android Device (192.168.1.x:5555)"]
    end
    DISC --> RUNNER
    EMU --> LD
    ACTOR --> GEOM
    GEOM --> RUNNER
    RUNNER --> LD
    RUNNER --> TCP
""",
    "IV_06_task_scheduler_multidevice_execution.mmd": """flowchart LR
    subgraph QUEUE["Project Task Queue"]
        T1["Task #1 [NV] Priority 1"]
        T2["Task #2 [PB] Priority 2"]
        T3["Task #3 [HĐ] Priority 3"]
    end
    subgraph SCHED["fp-scheduler::DeviceScheduler"]
        PQ["BinaryHeap Priority Queue"]
        LEASE["Per-Device Lease Table\\n(device_id -> LeaseHolder + Generation u64)"]
    end
    subgraph WORKERS["Isolated Tokio Device Workers"]
        W_A["Worker A: emulator-5554\\n(Lease Gen #14)"]
        W_B["Worker B: emulator-5556\\n(Lease Gen #08)"]
    end
    T1 & T2 & T3 --> PQ
    PQ --> LEASE
    LEASE -->|Exclusive Lease Gen #14| W_A
    LEASE -->|Exclusive Lease Gen #08| W_B
""",
    "IV_07_mini_tool_ui_schema_renderer.mmd": """flowchart LR
    SCHEMA["bundle/ui-schema.json\\n(Signed in Bundle)"] --> PARSER["fp-bundle-contract::MiniToolUiSchema"]
    MANIFEST["bundle/manifest.fpbundle.json"] --> MOD_MATRIX["5-State Module Matrix Evaluator"]
    PARSER --> THEME["1. Dynamic Window Title, Badge & Brand Colors\\n(e.g. #F59E0B JX2 vs #10B981 Kingdom SLG)"]
    PARSER --> COLS["2. Dynamic Device Table & OCR Columns\\n(Truthful Fallback: 'Chưa OCR (-)' until verified)"]
    PARSER --> CATS["3. Category Filter Tabs & Priority Task Checklist\\n([NV]/[PB]/[HĐ] or [Nông Nghiệp]/[Quân Sự])"]
    PARSER --> FIELDS["4. Per-Task Dynamic Settings Drawer\\n(boolean / number / select / text controls)"]
    MOD_MATRIX --> MOD_UI["5. Permitted Module Controls & Honest Technical Notes"]
""",
    "IV_08_mini_tool_build_release_pipeline.mmd": """flowchart LR
    S1["Step 1: Validate Project,\\nUI Schema & Workflows"] --> S2["Step 2: Asset Integrity &\\nPath Traversal Guard"]
    S2 --> S3["Step 3: Resolve 5-State\\nModule Matrix"]
    S3 --> S4["Step 4: Copy Dedicated\\nfp-mini-runtime.exe"]
    S4 --> S5["Step 5: Win32 UpdateResourceW\\nInject PE Icon + RT_RCDATA #7777 PubKey"]
    S5 --> S6["Step 6: Package bundle/ Assets\\n& ./bin/adb.exe Dependencies"]
    S6 --> S7["Step 7: Compute SHA-256 Table &\\nSign Ed25519 via External Keystore"]
    S7 --> S8["Step 8: Verify Signature Against\\nPE RT_RCDATA #7777 & Audit Runtime"]
""",
    "IV_09_security_architecture_ed25519_root_of_trust.mmd": """flowchart TB
    subgraph PRODUCER["Producer Machine (Studio / Build Time)"]
        KS["External Producer Keystore\\n%LOCALAPPDATA%\\FarmerPro\\ProducerKeys\\active_producer_ed25519.json\\n(32-byte SigningKey NEVER in repo or bundle)"]
        INJ["Step 5: Win32 UpdateResourceW\\nEmbed 32-byte VerifyingKey into PE .rsrc\\nRT_RCDATA (#10) Resource ID #7777"]
        SIGN["Step 7: RFC 8032 Ed25519 Signer\\nSigns Canonical SHA-256 Manifest Table"]
        AUTH["P5 Optional Post-Step: Windows Authenticode\\n(signtool.exe EV/OV Certificate signs entire .exe image)"]
    end
    subgraph RUNTIME["End-User Machine (Auto_<Game>.exe Startup)"]
        PE_READ["1. Read Trusted Public Key from Current Process PE\\nLoadLibraryExW + FindResourceW(RT_RCDATA, #7777)"]
        DISK_CHK["2. Cross-Check bundle/trusted_publisher.pub &\\nsignature.fpbundle.json publisherPublicKeyHex == PE Key"]
        ED_VER["3. Verify RFC 8032 Ed25519 Signature &\\nEvery File SHA-256 in bundle/"]
    end
    KS --> INJ
    KS --> SIGN
    INJ --> AUTH
    AUTH --> PE_READ
    SIGN --> DISK_CHK
    PE_READ --> DISK_CHK
    DISK_CHK --> ED_VER
""",
    "IV_10_studio_to_mini_tool_production_lifecycle.mmd": """flowchart LR
    subgraph CORE["Unchanged Shared Rust Core (14 Crates)"]
        CRATES["fp-domain · fp-adb · fp-device · fp-vision · fp-actions\\nfp-workflow-schema · fp-workflow-runtime · fp-scheduler\\nfp-storage · fp-security · fp-bundle-contract · fp-bundle-verify"]
    end
    subgraph STUDIO_PROJECTS["FarmerPro Studio Multi-Game Projects"]
        P_A["Game Project A: jx2-mobile\\n(2 Workflows, 4 PNGs, 5 Tasks, 5 Modules)"]
        P_B["Game Project B: kingdom-slg\\n(1 Workflow, 3 PNGs, 4 Tasks, 4 Modules)"]
        P_N["Game Project N: new-rpg-game\\n(Custom Workflows, PNGs, Tasks, Modules)"]
    end
    subgraph PACKAGER["fp-builder (No Rust Recompile Needed)"]
        BLD["MiniToolPackager::build_and_export_release()"]
    end
    subgraph RELEASES["Independent Commercial Mini Auto-Tools"]
        OUT_A["Auto_JX2_Mobile.exe (Amber UI, 5 Tasks)"]
        OUT_B["Auto_Kingdom_SLG.exe (Emerald UI, 4 Tasks)"]
        OUT_N["Auto_New_RPG.exe (Custom Theme & Tasks)"]
    end
    CRATES --> BLD
    P_A & P_B & P_N --> BLD
    BLD --> OUT_A & OUT_B & OUT_N
"""
}

for fname, content in MERMAID_SOURCES.items():
    with open(os.path.join(DIAGRAM_DIR, fname), "w", encoding="utf-8") as f:
        f.write(content.strip() + "\n")

print(f"Saved {len(MERMAID_SOURCES)} editable Mermaid (.mmd) files to {DIAGRAM_DIR}")
