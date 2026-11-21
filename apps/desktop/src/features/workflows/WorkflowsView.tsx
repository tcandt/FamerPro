import React, { useState } from 'react';
import {
  Plus,
  CheckCircle2,
  ArrowRight,
  Gamepad2,
  Search,
} from 'lucide-react';
import { jx2Workflows, Jx2WorkflowItem } from '../../data/jx2Workflows';
import { useWorkflowStore } from '../../stores/workflowStore';

export const WorkflowsView: React.FC<{ onOpenStudio: () => void }> = ({ onOpenStudio }) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const loadWorkflow = useWorkflowStore((state) => state.loadWorkflow);

  const categories = ['all', ...Array.from(new Set(jx2Workflows.map((w) => w.category)))];

  const filteredWorkflows = jx2Workflows.filter((wf) => {
    if (selectedCategory !== 'all' && wf.category !== selectedCategory) return false;
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return (
        wf.name.toLowerCase().includes(q) ||
        wf.category.toLowerCase().includes(q) ||
        wf.id.toLowerCase().includes(q) ||
        (wf.disclaimer || '').toLowerCase().includes(q)
      );
    }
    return true;
  });

  const handleSelectWorkflow = (wf: Jx2WorkflowItem) => {
    loadWorkflow(wf.data);
    onOpenStudio();
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-[#EDF3FC]">
              Kho Quy trình Tự động (Workflow Library)
            </h1>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF] font-semibold border border-[#2A3647] flex items-center gap-1">
              <Gamepad2 size={12} />
              <span>Võ Hiệp 2 Mobile (10 Luồng)</span>
            </span>
          </div>
          <p className="text-xs text-[#74869D] mt-1">
            Danh sách 10 quy trình tự động hóa tiêu chuẩn cho Võ Hiệp 2 Mobile trên giả lập LDPlayer / thiết bị thật.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => {
              // Create empty workflow
              loadWorkflow({
                workflowId: `wf-custom-${Date.now()}`,
                name: 'Quy trình Mới (Bản nháp)',
                graph: {
                  nodes: [
                    { id: 'n-start', type: 'flow.start', position: { x: 50, y: 50 }, config: {} },
                    { id: 'n-end', type: 'flow.end', position: { x: 400, y: 50 }, config: {} },
                  ],
                  edges: [
                    { id: 'e1', source: 'n-start', target: 'n-end' },
                  ],
                },
              });
              onOpenStudio();
            }}
            className="px-3.5 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
          >
            <Plus size={14} />
            <span>Tạo quy trình mới</span>
          </button>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex items-center justify-between gap-4 bg-[#16202F] p-3 rounded-xl border border-[#2A3647]">
        {/* Category Pills */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <button
            onClick={() => setSelectedCategory('all')}
            className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
              selectedCategory === 'all'
                ? 'bg-[#1878F5] text-white'
                : 'bg-[#202B3B] text-[#74869D] hover:text-[#EDF3FC]'
            }`}
          >
            Tất cả ({jx2Workflows.length})
          </button>
          {categories.filter((c) => c !== 'all').map((cat) => (
            <button
              key={cat}
              onClick={() => setSelectedCategory(cat)}
              className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition-colors ${
                selectedCategory === cat
                  ? 'bg-[#173355] text-[#38BDF8] border border-[#38BDF8]/40'
                  : 'bg-[#202B3B] text-[#74869D] hover:text-[#EDF3FC] border border-[#2A3647]'
              }`}
            >
              {cat}
            </button>
          ))}
        </div>

        {/* Search Input */}
        <div className="relative w-64">
          <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[#74869D]" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Tìm theo tên hoặc mã quy trình..."
            className="w-full pl-8 pr-3 py-1 bg-[#101722] border border-[#2A3647] rounded-md text-xs text-[#EDF3FC] placeholder-[#74869D] focus:outline-none focus:border-[#1878F5]"
          />
        </div>
      </div>

      {/* Workflows Grid */}
      <div className="grid grid-cols-3 gap-4">
        {filteredWorkflows.map((wf) => (
          <div
            key={wf.id}
            onClick={() => handleSelectWorkflow(wf)}
            className="p-4 rounded-xl bg-[#16202F] border border-[#2A3647] hover:border-[#1878F5] cursor-pointer transition-all flex flex-col justify-between h-48 group shadow-sm hover:shadow-md hover:bg-[#192434]"
          >
            <div>
              <div className="flex items-center justify-between text-xs mb-2.5">
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-semibold border border-[#38BDF8]/30">
                  {wf.category}
                </span>
                <span className="text-[11px] text-[#74869D] font-mono">{wf.version}</span>
              </div>
              <h3 className="text-sm font-bold text-[#EDF3FC] group-hover:text-[#38BDF8] transition-colors line-clamp-1">
                {wf.name}
              </h3>
              <p className="text-[11px] text-[#74869D] mt-1.5 line-clamp-2 leading-relaxed">
                {wf.disclaimer || `Tệp cấu hình: games/jx2mobile/${wf.file}`}
              </p>
            </div>

            <div className="flex items-center justify-between pt-3 border-t border-[#2A3647] text-xs">
              <div className="flex items-center gap-2">
                <span className="flex items-center gap-1 text-[#3BC38D] text-[11px] font-medium">
                  <CheckCircle2 size={12} />
                  <span>{wf.status}</span>
                </span>
                <span className="text-[#74869D] text-[11px] font-mono">
                  · {wf.nodesCount} nodes
                </span>
              </div>
              <span className="text-xs text-[#38BDF8] group-hover:translate-x-1 transition-transform flex items-center gap-1 font-semibold">
                <span>Mở Canvas</span>
                <ArrowRight size={13} />
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
