/* Auto-generated JX2 Mobile 10 Workflows Definition */
export interface Jx2WorkflowItem {
  file: string;
  id: string;
  name: string;
  category: string;
  version: string;
  nodesCount: number;
  status: string;
  disclaimer?: string;
  data: any;
}

export const jx2Workflows: Jx2WorkflowItem[] = [
  {
    "file": "01_start_and_login.json",
    "id": "wf-jx2-01-login",
    "name": "01. Khởi động và Đăng nhập (Chơi Ngay)",
    "category": "Khởi động & Đăng nhập",
    "version": "v1.0.0",
    "nodesCount": 11,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-01-login",
      "name": "01. Khởi động và Đăng nhập (Chơi Ngay)",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-launch",
            "type": "app.launch",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {
              "packageId": "com.blackhole.jx2mobile"
            }
          },
          {
            "id": "n-wait-load",
            "type": "control.wait",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "durationMs": 3000
            }
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-login",
            "type": "vision.find_image",
            "position": {
              "x": 1050,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_dang_nhap.png",
              "minConfidence": 0.85
            }
          },
          {
            "id": "n-tap-login",
            "type": "action.random_safe_tap",
            "position": {
              "x": 1050,
              "y": 250
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-wait-modal",
            "type": "control.wait",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "durationMs": 1500
            }
          },
          {
            "id": "n-find-choi-ngay",
            "type": "vision.find_image",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_choi_ngay.png",
              "minConfidence": 0.85
            }
          },
          {
            "id": "n-tap-choi-ngay",
            "type": "action.random_safe_tap",
            "position": {
              "x": 300,
              "y": 250
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-login",
            "type": "state.checkpoint",
            "position": {
              "x": 50,
              "y": 250
            },
            "config": {
              "name": "logged_in_guest"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 50,
              "y": 450
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-launch",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-launch",
            "sourcePort": "flow",
            "target": "n-wait-load",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-wait-load",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-login",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-find-login",
            "sourcePort": "flow",
            "target": "n-tap-login",
            "targetPort": "flow"
          },
          {
            "id": "e6",
            "source": "n-tap-login",
            "sourcePort": "flow",
            "target": "n-wait-modal",
            "targetPort": "flow"
          },
          {
            "id": "e7",
            "source": "n-wait-modal",
            "sourcePort": "flow",
            "target": "n-find-choi-ngay",
            "targetPort": "flow"
          },
          {
            "id": "e8",
            "source": "n-find-choi-ngay",
            "sourcePort": "flow",
            "target": "n-tap-choi-ngay",
            "targetPort": "flow"
          },
          {
            "id": "e9",
            "source": "n-tap-choi-ngay",
            "sourcePort": "flow",
            "target": "n-cp-login",
            "targetPort": "flow"
          },
          {
            "id": "e10",
            "source": "n-cp-login",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "02_character_creation.json",
    "id": "wf-jx2-02-char",
    "name": "02. Tạo nhân vật và Vào Game",
    "category": "Tạo nhân vật",
    "version": "v1.0.0",
    "nodesCount": 7,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-02-char",
      "name": "02. Tạo nhân vật và Vào Game",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-wait-char",
            "type": "control.wait",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {
              "durationMs": 2500
            }
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-vao-game",
            "type": "vision.find_image",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_vao_game.png",
              "minConfidence": 0.85
            }
          },
          {
            "id": "n-tap-vao-game",
            "type": "action.random_safe_tap",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-char",
            "type": "state.checkpoint",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {
              "name": "character_created"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 300,
              "y": 250
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-wait-char",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-wait-char",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-vao-game",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-find-vao-game",
            "sourcePort": "flow",
            "target": "n-tap-vao-game",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-tap-vao-game",
            "sourcePort": "flow",
            "target": "n-cp-char",
            "targetPort": "flow"
          },
          {
            "id": "e6",
            "source": "n-cp-char",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "03_tutorial_guidance.json",
    "id": "wf-jx2-03-tutorial",
    "name": "03. Tiếp nhận Chỉ dẫn Tân Thủ",
    "category": "Hướng dẫn tân thủ",
    "version": "v1.0.0",
    "nodesCount": 7,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-03-tutorial",
      "name": "03. Tiếp nhận Chỉ dẫn Tân Thủ",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-wait-intro",
            "type": "control.wait",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {
              "durationMs": 2000
            }
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-biet-roi",
            "type": "vision.find_image",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_biet_roi.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-biet-roi",
            "type": "action.random_safe_tap",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-tutorial",
            "type": "state.checkpoint",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {
              "name": "tutorial_intro_done"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 300,
              "y": 250
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-wait-intro",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-wait-intro",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-biet-roi",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-find-biet-roi",
            "sourcePort": "flow",
            "target": "n-tap-biet-roi",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-tap-biet-roi",
            "sourcePort": "flow",
            "target": "n-cp-tutorial",
            "targetPort": "flow"
          },
          {
            "id": "e6",
            "source": "n-cp-tutorial",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "04_npc_interaction.json",
    "id": "wf-jx2-04-npc",
    "name": "04. Tương tác NPC Dã Tẩu",
    "category": "Tương tác NPC",
    "version": "v1.0.0",
    "nodesCount": 7,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-04-npc",
      "name": "04. Tương tác NPC Dã Tẩu",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-quest-btn",
            "type": "vision.find_image",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_quest_tan_thu.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-quest",
            "type": "action.random_safe_tap",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-wait-move",
            "type": "control.wait",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "durationMs": 4000
            }
          },
          {
            "id": "n-cp-da-tau",
            "type": "state.checkpoint",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {
              "name": "arrived_at_da_tau"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 300,
              "y": 250
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-quest-btn",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-find-quest-btn",
            "sourcePort": "flow",
            "target": "n-tap-quest",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-tap-quest",
            "sourcePort": "flow",
            "target": "n-wait-move",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-wait-move",
            "sourcePort": "flow",
            "target": "n-cp-da-tau",
            "targetPort": "flow"
          },
          {
            "id": "e6",
            "source": "n-cp-da-tau",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "05_quest_acceptance.json",
    "id": "wf-jx2-05-accept",
    "name": "05. Nhận Nhiệm vụ Tân thủ",
    "category": "Tiếp nhận nhiệm vụ",
    "version": "v1.0.0",
    "nodesCount": 6,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-05-accept",
      "name": "05. Nhận Nhiệm vụ Tân thủ",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-tracker",
            "type": "vision.find_image",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/quest_tracker_tan_thu.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-tracker",
            "type": "action.random_safe_tap",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-accept",
            "type": "state.checkpoint",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "name": "quest_accepted"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-tracker",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-find-tracker",
            "sourcePort": "flow",
            "target": "n-tap-tracker",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-tap-tracker",
            "sourcePort": "flow",
            "target": "n-cp-accept",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-cp-accept",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "06_movement.json",
    "id": "wf-jx2-06-movement",
    "name": "06. Di chuyển Thành Đô",
    "category": "Di chuyển",
    "version": "v1.0.0",
    "nodesCount": 4,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-06-movement",
      "name": "06. Di chuyển Thành Đô",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-move-wait",
            "type": "control.wait",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {
              "durationMs": 3000
            }
          },
          {
            "id": "n-cp-moved",
            "type": "state.checkpoint",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "name": "movement_finished"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-move-wait",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-move-wait",
            "sourcePort": "flow",
            "target": "n-cp-moved",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-cp-moved",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "07_combat.json",
    "id": "wf-jx2-07-combat",
    "name": "07. Kích hoạt Tính năng Auto Chiến đấu",
    "category": "Chiến đấu",
    "version": "v1.0.0",
    "nodesCount": 6,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-07-combat",
      "name": "07. Kích hoạt Tính năng Auto Chiến đấu",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-auto",
            "type": "vision.find_image",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_auto.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-auto",
            "type": "action.random_safe_tap",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-combat",
            "type": "state.checkpoint",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "name": "auto_combat_enabled"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-auto",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-find-auto",
            "sourcePort": "flow",
            "target": "n-tap-auto",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-tap-auto",
            "sourcePort": "flow",
            "target": "n-cp-combat",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-cp-combat",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "08_quest_verification.json",
    "id": "wf-jx2-08-verify",
    "name": "08. Kiểm chứng và Nhận Thưởng",
    "category": "Xác minh & Thưởng",
    "version": "v1.0.0",
    "nodesCount": 6,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-08-verify",
      "name": "08. Kiểm chứng và Nhận Thưởng",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-capture",
            "type": "vision.capture",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-reward",
            "type": "vision.find_image",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_nhan_thuong.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-reward",
            "type": "action.random_safe_tap",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-verified",
            "type": "state.checkpoint",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "name": "quest_reward_claimed"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-capture",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-capture",
            "sourcePort": "flow",
            "target": "n-find-reward",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-find-reward",
            "sourcePort": "flow",
            "target": "n-tap-reward",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-tap-reward",
            "sourcePort": "flow",
            "target": "n-cp-verified",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-cp-verified",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "09_tutorial_completion.json",
    "id": "wf-jx2-09-complete",
    "name": "09. Hoàn thành Giai đoạn Tân Thủ",
    "category": "Hoàn tất tân thủ",
    "version": "v1.0.0",
    "nodesCount": 4,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-09-complete",
      "name": "09. Hoàn thành Giai đoạn Tân Thủ",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-wait-done",
            "type": "control.wait",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {
              "durationMs": 2000
            }
          },
          {
            "id": "n-cp-complete",
            "type": "state.checkpoint",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "name": "tutorial_phase_1_completed"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-wait-done",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-wait-done",
            "sourcePort": "flow",
            "target": "n-cp-complete",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-cp-complete",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  },
  {
    "file": "e2e_beginner_quest.json",
    "id": "wf-jx2-e2e-beginner",
    "name": "Toàn trình Nhiệm vụ Tân Thủ Võ Hiệp 2",
    "category": "E2E Toàn trình",
    "version": "v1.0.0",
    "nodesCount": 12,
    "status": "Hợp lệ",
    "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
    "data": {
      "schemaVersion": 1,
      "workflowId": "wf-jx2-e2e-beginner",
      "name": "Toàn trình Nhiệm vụ Tân Thủ Võ Hiệp 2",
      "status": "published",
      "disclaimer": "Live LDPlayer Võ Hiệp Truyền Kỳ 2 Mobile automation workflow",
      "graph": {
        "nodes": [
          {
            "id": "n-start",
            "type": "flow.start",
            "position": {
              "x": 50,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-launch",
            "type": "app.launch",
            "position": {
              "x": 300,
              "y": 50
            },
            "config": {
              "packageId": "com.blackhole.jx2mobile"
            }
          },
          {
            "id": "n-wait-load",
            "type": "control.wait",
            "position": {
              "x": 550,
              "y": 50
            },
            "config": {
              "durationMs": 3000
            }
          },
          {
            "id": "n-capture-1",
            "type": "vision.capture",
            "position": {
              "x": 800,
              "y": 50
            },
            "config": {}
          },
          {
            "id": "n-find-quest",
            "type": "vision.find_image",
            "position": {
              "x": 1050,
              "y": 50
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_quest_tan_thu.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-quest",
            "type": "action.random_safe_tap",
            "position": {
              "x": 1050,
              "y": 250
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-wait-move",
            "type": "control.wait",
            "position": {
              "x": 800,
              "y": 250
            },
            "config": {
              "durationMs": 4000
            }
          },
          {
            "id": "n-capture-2",
            "type": "vision.capture",
            "position": {
              "x": 550,
              "y": 250
            },
            "config": {}
          },
          {
            "id": "n-find-reward",
            "type": "vision.find_image",
            "position": {
              "x": 300,
              "y": 250
            },
            "config": {
              "templateRef": "games/jx2mobile/templates/btn_nhan_thuong.png",
              "minConfidence": 0.8
            }
          },
          {
            "id": "n-tap-reward",
            "type": "action.random_safe_tap",
            "position": {
              "x": 50,
              "y": 250
            },
            "config": {
              "marginRatio": 0.15
            }
          },
          {
            "id": "n-cp-reward",
            "type": "state.checkpoint",
            "position": {
              "x": 50,
              "y": 450
            },
            "config": {
              "name": "beginner_quest_completed"
            }
          },
          {
            "id": "n-end",
            "type": "flow.end",
            "position": {
              "x": 300,
              "y": 450
            },
            "config": {}
          }
        ],
        "edges": [
          {
            "id": "e1",
            "source": "n-start",
            "sourcePort": "flow",
            "target": "n-launch",
            "targetPort": "flow"
          },
          {
            "id": "e2",
            "source": "n-launch",
            "sourcePort": "flow",
            "target": "n-wait-load",
            "targetPort": "flow"
          },
          {
            "id": "e3",
            "source": "n-wait-load",
            "sourcePort": "flow",
            "target": "n-capture-1",
            "targetPort": "flow"
          },
          {
            "id": "e4",
            "source": "n-capture-1",
            "sourcePort": "flow",
            "target": "n-find-quest",
            "targetPort": "flow"
          },
          {
            "id": "e5",
            "source": "n-find-quest",
            "sourcePort": "flow",
            "target": "n-tap-quest",
            "targetPort": "flow"
          },
          {
            "id": "e6",
            "source": "n-tap-quest",
            "sourcePort": "flow",
            "target": "n-wait-move",
            "targetPort": "flow"
          },
          {
            "id": "e7",
            "source": "n-wait-move",
            "sourcePort": "flow",
            "target": "n-capture-2",
            "targetPort": "flow"
          },
          {
            "id": "e8",
            "source": "n-capture-2",
            "sourcePort": "flow",
            "target": "n-find-reward",
            "targetPort": "flow"
          },
          {
            "id": "e9",
            "source": "n-find-reward",
            "sourcePort": "flow",
            "target": "n-tap-reward",
            "targetPort": "flow"
          },
          {
            "id": "e10",
            "source": "n-tap-reward",
            "sourcePort": "flow",
            "target": "n-cp-reward",
            "targetPort": "flow"
          },
          {
            "id": "e11",
            "source": "n-cp-reward",
            "sourcePort": "flow",
            "target": "n-end",
            "targetPort": "flow"
          }
        ]
      },
      "runtimePolicy": {
        "oneActiveInputWorkflowPerDevice": true,
        "retry": {
          "maxAttempts": 3,
          "backoffMs": 1200
        },
        "onUnknownInputResult": "pause",
        "onFailure": "fail",
        "endPolicy": "continue"
      }
    }
  }
];
