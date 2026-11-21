import json
import os

os.makedirs('games/jx2mobile', exist_ok=True)
os.makedirs('games/jx2mobile/templates', exist_ok=True)

def create_workflow(wf_id, name, nodes, edges):
    return {
        "schemaVersion": 1,
        "workflowId": wf_id,
        "name": name,
        "status": "published",
        "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
        "graph": {
            "nodes": nodes,
            "edges": edges
        },
        "runtimePolicy": {
            "oneActiveInputWorkflowPerDevice": True,
            "retry": {
                "maxAttempts": 3,
                "backoffMs": 1200
            },
            "onUnknownInputResult": "pause",
            "onFailure": "fail",
            "endPolicy": "continue"
        }
    }

# 1. 01_start_and_login
w1_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-launch", "type": "app.launch", "position": {"x": 300, "y": 50}, "config": {"packageId": "com.blackhole.jx2mobile"}},
    {"id": "n-wait-load", "type": "control.wait", "position": {"x": 550, "y": 50}, "config": {"durationMs": 3000}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 800, "y": 50}, "config": {}},
    {"id": "n-find-login", "type": "vision.find_image", "position": {"x": 1050, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_dang_nhap.png", "minConfidence": 0.85}},
    {"id": "n-tap-login", "type": "action.random_safe_tap", "position": {"x": 1050, "y": 250}, "config": {"marginRatio": 0.15}},
    {"id": "n-wait-modal", "type": "control.wait", "position": {"x": 800, "y": 250}, "config": {"durationMs": 1500}},
    {"id": "n-find-choi-ngay", "type": "vision.find_image", "position": {"x": 550, "y": 250}, "config": {"templateRef": "games/jx2mobile/templates/btn_choi_ngay.png", "minConfidence": 0.85}},
    {"id": "n-tap-choi-ngay", "type": "action.random_safe_tap", "position": {"x": 300, "y": 250}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-login", "type": "state.checkpoint", "position": {"x": 50, "y": 250}, "config": {"name": "logged_in_guest"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 50, "y": 450}, "config": {}}
]
w1_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow"},
    {"id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-wait-load", "targetPort": "flow"},
    {"id": "e3", "source": "n-wait-load", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e4", "source": "n-capture", "sourcePort": "flow", "target": "n-find-login", "targetPort": "flow"},
    {"id": "e5", "source": "n-find-login", "sourcePort": "flow", "target": "n-tap-login", "targetPort": "flow"},
    {"id": "e6", "source": "n-tap-login", "sourcePort": "flow", "target": "n-wait-modal", "targetPort": "flow"},
    {"id": "e7", "source": "n-wait-modal", "sourcePort": "flow", "target": "n-find-choi-ngay", "targetPort": "flow"},
    {"id": "e8", "source": "n-find-choi-ngay", "sourcePort": "flow", "target": "n-tap-choi-ngay", "targetPort": "flow"},
    {"id": "e9", "source": "n-tap-choi-ngay", "sourcePort": "flow", "target": "n-cp-login", "targetPort": "flow"},
    {"id": "e10", "source": "n-cp-login", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 2. 02_character_creation
w2_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-wait-char", "type": "control.wait", "position": {"x": 300, "y": 50}, "config": {"durationMs": 2500}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 550, "y": 50}, "config": {}},
    {"id": "n-find-vao-game", "type": "vision.find_image", "position": {"x": 800, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_vao_game.png", "minConfidence": 0.85}},
    {"id": "n-tap-vao-game", "type": "action.random_safe_tap", "position": {"x": 800, "y": 250}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-char", "type": "state.checkpoint", "position": {"x": 550, "y": 250}, "config": {"name": "character_created"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 300, "y": 250}, "config": {}}
]
w2_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-wait-char", "targetPort": "flow"},
    {"id": "e2", "source": "n-wait-char", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e3", "source": "n-capture", "sourcePort": "flow", "target": "n-find-vao-game", "targetPort": "flow"},
    {"id": "e4", "source": "n-find-vao-game", "sourcePort": "flow", "target": "n-tap-vao-game", "targetPort": "flow"},
    {"id": "e5", "source": "n-tap-vao-game", "sourcePort": "flow", "target": "n-cp-char", "targetPort": "flow"},
    {"id": "e6", "source": "n-cp-char", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 3. 03_tutorial_guidance
w3_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-wait-intro", "type": "control.wait", "position": {"x": 300, "y": 50}, "config": {"durationMs": 2000}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 550, "y": 50}, "config": {}},
    {"id": "n-find-biet-roi", "type": "vision.find_image", "position": {"x": 800, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_biet_roi.png", "minConfidence": 0.80}},
    {"id": "n-tap-biet-roi", "type": "action.random_safe_tap", "position": {"x": 800, "y": 250}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-tutorial", "type": "state.checkpoint", "position": {"x": 550, "y": 250}, "config": {"name": "tutorial_intro_done"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 300, "y": 250}, "config": {}}
]
w3_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-wait-intro", "targetPort": "flow"},
    {"id": "e2", "source": "n-wait-intro", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e3", "source": "n-capture", "sourcePort": "flow", "target": "n-find-biet-roi", "targetPort": "flow"},
    {"id": "e4", "source": "n-find-biet-roi", "sourcePort": "flow", "target": "n-tap-biet-roi", "targetPort": "flow"},
    {"id": "e5", "source": "n-tap-biet-roi", "sourcePort": "flow", "target": "n-cp-tutorial", "targetPort": "flow"},
    {"id": "e6", "source": "n-cp-tutorial", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 4. 04_npc_interaction
w4_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 300, "y": 50}, "config": {}},
    {"id": "n-find-quest-btn", "type": "vision.find_image", "position": {"x": 550, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_quest_tan_thu.png", "minConfidence": 0.80}},
    {"id": "n-tap-quest", "type": "action.random_safe_tap", "position": {"x": 800, "y": 50}, "config": {"marginRatio": 0.15}},
    {"id": "n-wait-move", "type": "control.wait", "position": {"x": 800, "y": 250}, "config": {"durationMs": 4000}},
    {"id": "n-cp-da-tau", "type": "state.checkpoint", "position": {"x": 550, "y": 250}, "config": {"name": "arrived_at_da_tau"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 300, "y": 250}, "config": {}}
]
w4_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e2", "source": "n-capture", "sourcePort": "flow", "target": "n-find-quest-btn", "targetPort": "flow"},
    {"id": "e3", "source": "n-find-quest-btn", "sourcePort": "flow", "target": "n-tap-quest", "targetPort": "flow"},
    {"id": "e4", "source": "n-tap-quest", "sourcePort": "flow", "target": "n-wait-move", "targetPort": "flow"},
    {"id": "e5", "source": "n-wait-move", "sourcePort": "flow", "target": "n-cp-da-tau", "targetPort": "flow"},
    {"id": "e6", "source": "n-cp-da-tau", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 5. 05_quest_acceptance
w5_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 300, "y": 50}, "config": {}},
    {"id": "n-find-tracker", "type": "vision.find_image", "position": {"x": 550, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/quest_tracker_tan_thu.png", "minConfidence": 0.80}},
    {"id": "n-tap-tracker", "type": "action.random_safe_tap", "position": {"x": 800, "y": 50}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-accept", "type": "state.checkpoint", "position": {"x": 800, "y": 250}, "config": {"name": "quest_accepted"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 550, "y": 250}, "config": {}}
]
w5_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e2", "source": "n-capture", "sourcePort": "flow", "target": "n-find-tracker", "targetPort": "flow"},
    {"id": "e3", "source": "n-find-tracker", "sourcePort": "flow", "target": "n-tap-tracker", "targetPort": "flow"},
    {"id": "e4", "source": "n-tap-tracker", "sourcePort": "flow", "target": "n-cp-accept", "targetPort": "flow"},
    {"id": "e5", "source": "n-cp-accept", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 6. 06_movement
w6_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-move-wait", "type": "control.wait", "position": {"x": 300, "y": 50}, "config": {"durationMs": 3000}},
    {"id": "n-cp-moved", "type": "state.checkpoint", "position": {"x": 550, "y": 50}, "config": {"name": "movement_finished"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 800, "y": 50}, "config": {}}
]
w6_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-move-wait", "targetPort": "flow"},
    {"id": "e2", "source": "n-move-wait", "sourcePort": "flow", "target": "n-cp-moved", "targetPort": "flow"},
    {"id": "e3", "source": "n-cp-moved", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 7. 07_combat
w7_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 300, "y": 50}, "config": {}},
    {"id": "n-find-auto", "type": "vision.find_image", "position": {"x": 550, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_auto.png", "minConfidence": 0.80}},
    {"id": "n-tap-auto", "type": "action.random_safe_tap", "position": {"x": 800, "y": 50}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-combat", "type": "state.checkpoint", "position": {"x": 800, "y": 250}, "config": {"name": "auto_combat_enabled"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 550, "y": 250}, "config": {}}
]
w7_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e2", "source": "n-capture", "sourcePort": "flow", "target": "n-find-auto", "targetPort": "flow"},
    {"id": "e3", "source": "n-find-auto", "sourcePort": "flow", "target": "n-tap-auto", "targetPort": "flow"},
    {"id": "e4", "source": "n-tap-auto", "sourcePort": "flow", "target": "n-cp-combat", "targetPort": "flow"},
    {"id": "e5", "source": "n-cp-combat", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 8. 08_quest_verification
w8_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-capture", "type": "vision.capture", "position": {"x": 300, "y": 50}, "config": {}},
    {"id": "n-find-reward", "type": "vision.find_image", "position": {"x": 550, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_nhan_thuong.png", "minConfidence": 0.80}},
    {"id": "n-tap-reward", "type": "action.random_safe_tap", "position": {"x": 800, "y": 50}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-verified", "type": "state.checkpoint", "position": {"x": 800, "y": 250}, "config": {"name": "quest_reward_claimed"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 550, "y": 250}, "config": {}}
]
w8_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow"},
    {"id": "e2", "source": "n-capture", "sourcePort": "flow", "target": "n-find-reward", "targetPort": "flow"},
    {"id": "e3", "source": "n-find-reward", "sourcePort": "flow", "target": "n-tap-reward", "targetPort": "flow"},
    {"id": "e4", "source": "n-tap-reward", "sourcePort": "flow", "target": "n-cp-verified", "targetPort": "flow"},
    {"id": "e5", "source": "n-cp-verified", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 9. 09_tutorial_completion
w9_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-wait-done", "type": "control.wait", "position": {"x": 300, "y": 50}, "config": {"durationMs": 2000}},
    {"id": "n-cp-complete", "type": "state.checkpoint", "position": {"x": 550, "y": 50}, "config": {"name": "tutorial_phase_1_completed"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 800, "y": 50}, "config": {}}
]
w9_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-wait-done", "targetPort": "flow"},
    {"id": "e2", "source": "n-wait-done", "sourcePort": "flow", "target": "n-cp-complete", "targetPort": "flow"},
    {"id": "e3", "source": "n-cp-complete", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

# 10. End-to-end full quest workflow
we2e_nodes = [
    {"id": "n-start", "type": "flow.start", "position": {"x": 50, "y": 50}, "config": {}},
    {"id": "n-launch", "type": "app.launch", "position": {"x": 300, "y": 50}, "config": {"packageId": "com.blackhole.jx2mobile"}},
    {"id": "n-wait-load", "type": "control.wait", "position": {"x": 550, "y": 50}, "config": {"durationMs": 3000}},
    {"id": "n-capture-1", "type": "vision.capture", "position": {"x": 800, "y": 50}, "config": {}},
    {"id": "n-find-quest", "type": "vision.find_image", "position": {"x": 1050, "y": 50}, "config": {"templateRef": "games/jx2mobile/templates/btn_quest_tan_thu.png", "minConfidence": 0.80}},
    {"id": "n-tap-quest", "type": "action.random_safe_tap", "position": {"x": 1050, "y": 250}, "config": {"marginRatio": 0.15}},
    {"id": "n-wait-move", "type": "control.wait", "position": {"x": 800, "y": 250}, "config": {"durationMs": 4000}},
    {"id": "n-capture-2", "type": "vision.capture", "position": {"x": 550, "y": 250}, "config": {}},
    {"id": "n-find-reward", "type": "vision.find_image", "position": {"x": 300, "y": 250}, "config": {"templateRef": "games/jx2mobile/templates/btn_nhan_thuong.png", "minConfidence": 0.80}},
    {"id": "n-tap-reward", "type": "action.random_safe_tap", "position": {"x": 50, "y": 250}, "config": {"marginRatio": 0.15}},
    {"id": "n-cp-reward", "type": "state.checkpoint", "position": {"x": 50, "y": 450}, "config": {"name": "beginner_quest_completed"}},
    {"id": "n-end", "type": "flow.end", "position": {"x": 300, "y": 450}, "config": {}}
]
we2e_edges = [
    {"id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow"},
    {"id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-wait-load", "targetPort": "flow"},
    {"id": "e3", "source": "n-wait-load", "sourcePort": "flow", "target": "n-capture-1", "targetPort": "flow"},
    {"id": "e4", "source": "n-capture-1", "sourcePort": "flow", "target": "n-find-quest", "targetPort": "flow"},
    {"id": "e5", "source": "n-find-quest", "sourcePort": "flow", "target": "n-tap-quest", "targetPort": "flow"},
    {"id": "e6", "source": "n-tap-quest", "sourcePort": "flow", "target": "n-wait-move", "targetPort": "flow"},
    {"id": "e7", "source": "n-wait-move", "sourcePort": "flow", "target": "n-capture-2", "targetPort": "flow"},
    {"id": "e8", "source": "n-capture-2", "sourcePort": "flow", "target": "n-find-reward", "targetPort": "flow"},
    {"id": "e9", "source": "n-find-reward", "sourcePort": "flow", "target": "n-tap-reward", "targetPort": "flow"},
    {"id": "e10", "source": "n-tap-reward", "sourcePort": "flow", "target": "n-cp-reward", "targetPort": "flow"},
    {"id": "e11", "source": "n-cp-reward", "sourcePort": "flow", "target": "n-end", "targetPort": "flow"}
]

files = [
    ("games/jx2mobile/01_start_and_login.json", "wf-jx2-01-login", "01. Khởi động và Đăng nhập (Chơi Ngay)", w1_nodes, w1_edges),
    ("games/jx2mobile/02_character_creation.json", "wf-jx2-02-char", "02. Tạo nhân vật và Vào Game", w2_nodes, w2_edges),
    ("games/jx2mobile/03_tutorial_guidance.json", "wf-jx2-03-tutorial", "03. Tiếp nhận Chỉ dẫn Tân Thủ", w3_nodes, w3_edges),
    ("games/jx2mobile/04_npc_interaction.json", "wf-jx2-04-npc", "04. Tương tác NPC Dã Tẩu", w4_nodes, w4_edges),
    ("games/jx2mobile/05_quest_acceptance.json", "wf-jx2-05-accept", "05. Nhận Nhiệm vụ Tân thủ", w5_nodes, w5_edges),
    ("games/jx2mobile/06_movement.json", "wf-jx2-06-movement", "06. Di chuyển Thành Đô", w6_nodes, w6_edges),
    ("games/jx2mobile/07_combat.json", "wf-jx2-07-combat", "07. Kích hoạt Tính năng Auto Chiến đấu", w7_nodes, w7_edges),
    ("games/jx2mobile/08_quest_verification.json", "wf-jx2-08-verify", "08. Kiểm chứng và Nhận Thưởng", w8_nodes, w8_edges),
    ("games/jx2mobile/09_tutorial_completion.json", "wf-jx2-09-complete", "09. Hoàn thành Giai đoạn Tân Thủ", w9_nodes, w9_edges),
    ("games/jx2mobile/e2e_beginner_quest.json", "wf-jx2-e2e-beginner", "Toàn trình Nhiệm vụ Tân Thủ Võ Hiệp 2", we2e_nodes, we2e_edges),
]

for path, wf_id, name, nodes, edges in files:
    doc = create_workflow(wf_id, name, nodes, edges)
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(doc, f, indent=2, ensure_ascii=False)
    print(f"Generated: {path}")
