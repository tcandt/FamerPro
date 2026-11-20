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
      items: [
        { type: 'flow.start', label: 'Bắt đầu luồng', category: 'flow' as const },
        { type: 'flow.end', label: 'Kết thúc luồng', category: 'flow' as const },
        { type: 'logic.wait', label: 'Chờ đợi (Wait)', category: 'logic' as const },
        { type: 'logic.loop', label: 'Vòng lặp giới hạn', category: 'logic' as const },
      ],
    },
    {
      id: 'app',
      title: 'Ứng dụng (App)',
      icon: Rocket,
      items: [
        { type: 'app.launch', label: 'Mở ứng dụng (Launch)', category: 'app' as const },
        { type: 'app.stop', label: 'Dừng ứng dụng (Stop)', category: 'app' as const },
      ],
    },
    {
      id: 'vision',
      title: 'Thị giác máy (Vision)',
      icon: Camera,
      items: [
        { type: 'vision.capture', label: 'Chụp ảnh màn hình', category: 'vision' as const },
        { type: 'vision.find_image', label: 'Tìm ảnh mẫu (Template)', category: 'vision' as const },
        { type: 'vision.ocr_text', label: 'Nhận dạng chữ (OCR)', category: 'vision' as const },
      ],
    },
    {
      id: 'action',
      title: 'Thao tác an toàn (Action)',
      icon: HandMetal,
      items: [
        { type: 'action.random_safe_tap', label: 'Random Safe Tap', category: 'action' as const },
        { type: 'action.swipe_region', label: 'Vuốt màn hình (Swipe)', category: 'action' as const },
      ],
    },
    {
      id: 'verify',
      title: 'Xác minh (Verification)',
      icon: ShieldCheck,
      items: [
        { type: 'verify.wait_for_image', label: 'Chờ ảnh xuất hiện', category: 'verify' as const },
        { type: 'verify.check_bounds', label: 'Kiểm tra vùng UI', category: 'verify' as const },
      ],
    },
    {
      id: 'state',
      title: 'Trạng thái (State)',
      icon: Bookmark,
      items: [
        { type: 'state.checkpoint', label: 'Lưu Checkpoint', category: 'state' as const },
        { type: 'data.set_variable', label: 'Gán biến (Set Var)', category: 'state' as const },
      ],
    },
  ];

  const filteredGroups = paletteGroups
    .map((grp) => ({
      ...grp,
      items: grp.items.filter(
        (item) =>
          item.label.toLowerCase().includes(searchTerm.toLowerCase()) ||
          item.type.toLowerCase().includes(searchTerm.toLowerCase())
      ),
    }))
    .filter((grp) => grp.items.length > 0);

  return (
    <div className="w-[240px] bg-[#161E29] border-r border-[#2A3647] flex flex-col h-full select-none shrink-0 z-20">
      <div className="p-3 border-b border-[#2A3647]">
        <h3 className="text-xs font-semibold text-[#EDF3FC] mb-2">Thư viện Node</h3>
        <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1 text-xs">
          <Search size={13} className="text-[#74869D]" />
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
          return (
            <div key={group.id} className="rounded border border-[#2A3647] bg-[#192230] overflow-hidden">
              <button
                onClick={() => toggleCategory(group.id)}
                className="w-full flex items-center justify-between px-2.5 py-1.5 text-xs font-medium text-[#A9B8CC] hover:text-[#EDF3FC] hover:bg-[#202B3B] transition-colors"
              >
                <span>{group.title}</span>
                {isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </button>

              {isOpen && (
                <div className="p-1 space-y-1 bg-[#121A26]">
                  {group.items.map((item) => (
                    <div
                      key={item.type}
                      className="group flex items-center justify-between p-1.5 rounded hover:bg-[#192230] border border-transparent hover:border-[#2A3647] cursor-pointer text-xs"
                      onClick={() => addNode(item.type, item.category, item.label)}
                    >
                      <div className="flex items-center gap-2 truncate">
                        <span className="text-[#EDF3FC] truncate">{item.label}</span>
                      </div>
                      <Plus
                        size={13}
                        className="text-[#74869D] group-hover:text-[#1878F5] shrink-0"
                      />
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
