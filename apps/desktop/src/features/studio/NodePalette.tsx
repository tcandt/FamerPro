import React, { useState } from 'react';
import {
  Search,
  Play,
  Rocket,
  Camera,
  HandMetal,
  ShieldCheck,
  Bookmark,
  ChevronDown,
  ChevronRight,
  Plus,
  Square,
  Clock,
  Repeat,
  ScanText,
  MousePointerClick,
  Move,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';

export const NodePalette: React.FC = () => {
  const [searchTerm, setSearchTerm] = useState('');
  const [openCategories, setOpenCategories] = useState<Record<string, boolean>>({
    flow: true,
    app: true,
    vision: true,
    action: true,
    verify: true,
    state: true,
  });

  const { addNode } = useWorkflowStore();

  const toggleCategory = (cat: string) => {
    setOpenCategories((prev) => ({ ...prev, [cat]: !prev[cat] }));
  };

  const paletteGroups = [
    {
      id: 'flow',
      title: 'Điều khiển luồng (Flow)',
      icon: Play,
      iconColor: 'text-[#7BA6D6]',
      items: [
        {
          type: 'flow.start',
          label: 'Bắt đầu luồng',
          category: 'flow' as const,
          desc: 'Điểm khởi tạo quy trình',
          icon: Play,
        },
        {
          type: 'flow.end',
          label: 'Kết thúc luồng',
          category: 'flow' as const,
          desc: 'Điểm kết thúc phiên chạy',
          icon: Square,
        },
        {
          type: 'logic.wait',
          label: 'Chờ đợi (Delay)',
          category: 'logic' as const,
          desc: 'Chờ N mili giây ngẫu nhiên',
          icon: Clock,
        },
        {
          type: 'logic.loop',
          label: 'Vòng lặp (Loop)',
          category: 'logic' as const,
          desc: 'Lặp lại khối với giới hạn đếm',
          icon: Repeat,
        },
      ],
    },
    {
      id: 'app',
      title: 'Ứng dụng Android (App)',
      icon: Rocket,
      iconColor: 'text-[#1878F5]',
      items: [
        {
          type: 'app.launch',
          label: 'Mở ứng dụng (Launch)',
          category: 'app' as const,
          desc: 'Khởi chạy package game qua ADB',
          icon: Rocket,
        },
        {
          type: 'app.stop',
          label: 'Dừng ứng dụng (Force Stop)',
          category: 'app' as const,
          desc: 'Tắt nóng ứng dụng tránh treo máy',
          icon: Square,
        },
      ],
    },
    {
      id: 'vision',
      title: 'Thị giác máy (Vision)',
      icon: Camera,
      iconColor: 'text-[#B291F7]',
      items: [
        {
          type: 'vision.capture',
          label: 'Chụp ảnh màn hình',
          category: 'vision' as const,
          desc: 'Lấy snapshot PNG độ trễ thấp',
          icon: Camera,
        },
        {
          type: 'vision.find_image',
          label: 'Tìm ảnh mẫu (Template)',
          category: 'vision' as const,
          desc: 'So khớp mẫu ảnh đa tỉ lệ SSD',
          icon: Search,
        },
        {
          type: 'vision.ocr_text',
          label: 'Nhận dạng chữ (OCR)',
          category: 'vision' as const,
          desc: 'Đọc số vàng, level, thông báo',
          icon: ScanText,
        },
      ],
    },
    {
      id: 'action',
      title: 'Thao tác an toàn (Action)',
      icon: HandMetal,
      iconColor: 'text-[#38BDF8]',
      items: [
        {
          type: 'action.random_safe_tap',
          label: 'Safe Random Tap',
          category: 'action' as const,
          desc: 'Click co biên + chống tọa độ cố định',
          icon: MousePointerClick,
        },
        {
          type: 'action.swipe_region',
          label: 'Vuốt màn hình (Swipe)',
          category: 'action' as const,
          desc: 'Vuốt cong Bezier ngẫu nhiên',
          icon: Move,
        },
      ],
    },
    {
      id: 'verify',
      title: 'Xác minh (Verification)',
      icon: ShieldCheck,
      iconColor: 'text-[#3BC38D]',
      items: [
        {
          type: 'verify.wait_for_image',
          label: 'Chờ ảnh xuất hiện',
          category: 'verify' as const,
          desc: 'Xác nhận trạng thái sau click',
          icon: ShieldCheck,
        },
        {
          type: 'verify.check_bounds',
          label: 'Kiểm tra vùng UI',
          category: 'verify' as const,
          desc: 'Đảm bảo cửa sổ không bị lệch',
          icon: ShieldCheck,
        },
      ],
    },
    {
      id: 'state',
      title: 'Mốc trạng thái (State)',
      icon: Bookmark,
      iconColor: 'text-[#E7B15B]',
      items: [
        {
          type: 'state.checkpoint',
          label: 'Lưu Checkpoint',
          category: 'state' as const,
          desc: 'Lưu điểm an toàn để resume',
          icon: Bookmark,
        },
        {
          type: 'state.set_var',
          label: 'Gán biến Runtime',
          category: 'state' as const,
          desc: 'Xuất giá trị ra Mini Tool UI',
          icon: Bookmark,
        },
        {
          type: 'state.increment_var',
          label: 'Tăng bộ đếm Runtime',
          category: 'state' as const,
          desc: 'Cộng dồn biến đếm (số lượt/vàng)',
          icon: Bookmark,
        },
      ],
    },
  ];

  const filteredGroups = paletteGroups
    .map((grp) => ({
      ...grp,
      items: grp.items.filter(
        (item) =>
          item.label.toLowerCase().includes(searchTerm.toLowerCase()) ||
          item.type.toLowerCase().includes(searchTerm.toLowerCase()) ||
          item.desc.toLowerCase().includes(searchTerm.toLowerCase())
      ),
    }))
    .filter((grp) => grp.items.length > 0);

  return (
    <div className="w-[210px] xl:w-[250px] bg-[#141C28] border-r border-[#2A3647] flex flex-col h-full select-none shrink-0 z-20">
      <div className="p-3 border-b border-[#2A3647]">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-xs font-bold text-[#EDF3FC] tracking-wide">THƯ VIỆN NODE</h3>
          <span className="text-[10px] text-[#74869D] font-mono">12 nodes</span>
        </div>
        <div className="flex items-center gap-2 bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1.5 text-xs">
          <Search size={13} className="text-[#74869D] shrink-0" />
          <input
            type="text"
            placeholder="Tìm kiếm node..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full bg-transparent text-[#EDF3FC] placeholder-[#74869D] outline-none text-xs"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {filteredGroups.map((group) => {
          const isOpen = openCategories[group.id] ?? true;
          const GroupIcon = group.icon;
          return (
            <div
              key={group.id}
              className="rounded-lg border border-[#2A3647] bg-[#17202E] overflow-hidden"
            >
              <button
                onClick={() => toggleCategory(group.id)}
                className="w-full flex items-center justify-between px-3 py-2 text-xs font-semibold text-[#A9B8CC] hover:text-[#EDF3FC] hover:bg-[#1E293B] transition-colors"
              >
                <div className="flex items-center gap-2">
                  <GroupIcon size={14} className={group.iconColor} />
                  <span>{group.title}</span>
                </div>
                {isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </button>

              {isOpen && (
                <div className="p-1.5 space-y-1 bg-[#101722]/60 border-t border-[#2A3647]/50">
                  {group.items.map((item) => {
                    const ItemIcon = item.icon;
                    return (
                      <div
                        key={item.type}
                        className="group flex items-center justify-between p-2 rounded-md hover:bg-[#192434] border border-transparent hover:border-[#2A3647] cursor-pointer text-xs transition-all"
                        onClick={() => addNode(item.type, item.category, item.label)}
                        title={`Thêm node ${item.label}`}
                      >
                        <div className="flex items-start gap-2 min-w-0">
                          <ItemIcon size={14} className="mt-0.5 text-[#74869D] group-hover:text-[#38BDF8] shrink-0" />
                          <div className="min-w-0">
                            <div className="text-[#EDF3FC] font-medium truncate text-[12px]">
                              {item.label}
                            </div>
                            <div className="text-[10px] text-[#74869D] truncate">
                              {item.desc}
                            </div>
                          </div>
                        </div>
                        <Plus
                          size={14}
                          className="text-[#74869D] group-hover:text-[#38BDF8] shrink-0 ml-1 opacity-0 group-hover:opacity-100 transition-opacity"
                        />
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
