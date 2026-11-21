import React, { useState } from 'react';
import {
  KeyRound,
  Plus,
  ShieldCheck,
  Search,
  Clock,
  AlertCircle,
  Play,
  RotateCw,
} from 'lucide-react';

interface AccountRecord {
  id: string;
  alias: string;
  gameProfileId: string;
  assignedDevice: string;
  secretRef: string;
  status: 'ready' | 'in_session' | 'cooldown' | 'expired';
  cooldownUntil?: string;
  goldFarmed: number;
  level: number;
  lastActive: string;
}

export const AccountsView: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [selectedAccountId, setSelectedAccountId] = useState<string>('acc-01');
  const [showAddModal, setShowAddModal] = useState(false);

  const [accounts] = useState<AccountRecord[]>([
    {
      id: 'acc-01',
      alias: 'Farmer_Main_01',
      gameProfileId: 'Hero_Knight_Lv42',
      assignedDevice: 'Pixel 6 — Máy 01',
      secretRef: 'CRED_MGR://farmerpro/acc_main_01',
      status: 'in_session',
      goldFarmed: 145000,
      level: 42,
      lastActive: 'Vừa xong',
    },
    {
      id: 'acc-02',
      alias: 'Farmer_Alt_02',
      gameProfileId: 'Mage_Frost_Lv38',
      assignedDevice: 'Galaxy S22 — Máy 02',
      secretRef: 'CRED_MGR://farmerpro/acc_alt_02',
      status: 'ready',
      goldFarmed: 89000,
      level: 38,
      lastActive: '12 phút trước',
    },
    {
      id: 'acc-03',
      alias: 'Farmer_Alt_03',
      gameProfileId: 'Ranger_Wind_Lv31',
      assignedDevice: 'Chưa gán máy',
      secretRef: 'CRED_MGR://farmerpro/acc_alt_03',
      status: 'cooldown',
      cooldownUntil: 'Còn 35 phút',
      goldFarmed: 42000,
      level: 31,
      lastActive: '45 phút trước',
    },
    {
      id: 'acc-04',
      alias: 'Farmer_Backup_04',
      gameProfileId: 'Priest_Light_Lv25',
      assignedDevice: 'Chưa gán máy',
      secretRef: 'CRED_MGR://farmerpro/acc_backup_04',
      status: 'expired',
      goldFarmed: 12000,
      level: 25,
      lastActive: '2 ngày trước',
    },
  ]);

  const selectedAccount = accounts.find((a) => a.id === selectedAccountId) || accounts[0];

  const filteredAccounts = accounts.filter((acc) => {
    if (filterStatus !== 'all' && acc.status !== filterStatus) return false;
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return (
        acc.alias.toLowerCase().includes(q) ||
        acc.gameProfileId.toLowerCase().includes(q) ||
        acc.assignedDevice.toLowerCase().includes(q)
      );
    }
    return true;
  });

  const renderStatusChip = (status: AccountRecord['status']) => {
    switch (status) {
      case 'in_session':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#173355] text-[#38BDF8] border border-[#38BDF8]/40 flex items-center gap-1 w-fit">
            <span className="w-1.5 h-1.5 rounded-full bg-[#38BDF8] animate-pulse" />
            <span>Đang trong phiên</span>
          </span>
        );
      case 'ready':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40 flex items-center gap-1 w-fit">
            <span className="w-1.5 h-1.5 rounded-full bg-[#3BC38D]" />
            <span>Sẵn sàng</span>
          </span>
        );
      case 'cooldown':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#3B2D17] text-[#E7B15B] border border-[#E7B15B]/40 flex items-center gap-1 w-fit">
            <Clock size={10} />
            <span>Chờ hồi phục</span>
          </span>
        );
      case 'expired':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#381E24] text-[#F27883] border border-[#F27883]/40 flex items-center gap-1 w-fit">
            <AlertCircle size={10} />
            <span>Cần đăng nhập lại</span>
          </span>
        );
    }
  };

  return (
    <div className="flex-1 flex overflow-hidden bg-[#101722] text-[#EDF3FC]">
      {/* Left: Accounts Table & Filter */}
      <div className="flex-1 flex flex-col border-r border-[#2A3647] overflow-hidden">
        {/* Header Bar */}
        <div className="p-4 border-b border-[#2A3647] space-y-3 bg-[#141C28]">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-sm font-bold text-[#EDF3FC] tracking-wide">
                  Quản lý Tài khoản (Account Vault)
                </h1>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
                  {accounts.length} tài khoản
                </span>
              </div>
              <p className="text-xs text-[#74869D] mt-0.5">
                Bảo vệ bí mật thông qua Windows Credential Manager. Tuyệt đối không lưu mật khẩu plaintext trong JSON hoặc SQLite.
              </p>
            </div>

            <button
              onClick={() => setShowAddModal(true)}
              className="px-3 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md shadow-sm flex items-center gap-1.5 transition-colors cursor-pointer"
            >
              <Plus size={13} />
              <span>Thêm tài khoản</span>
            </button>
          </div>

          {/* Quick Metrics Bar */}
          <div className="grid grid-cols-4 gap-2 text-xs">
            <div className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between">
              <span className="text-[#74869D]">Tổng số:</span>
              <span className="font-bold text-[#EDF3FC] font-mono">{accounts.length}</span>
            </div>
            <div className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between">
              <span className="text-[#74869D]">Đang chạy:</span>
              <span className="font-bold text-[#38BDF8] font-mono">
                {accounts.filter((a) => a.status === 'in_session').length}
              </span>
            </div>
            <div className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between">
              <span className="text-[#74869D]">Sẵn sàng:</span>
              <span className="font-bold text-[#3BC38D] font-mono">
                {accounts.filter((a) => a.status === 'ready').length}
              </span>
            </div>
            <div className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between">
              <span className="text-[#74869D]">Cần xử lý:</span>
              <span className="font-bold text-[#E7B15B] font-mono">
                {accounts.filter((a) => a.status === 'cooldown' || a.status === 'expired').length}
              </span>
            </div>
          </div>

          {/* Search & Tabs */}
          <div className="flex items-center justify-between gap-3 pt-1">
            <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs flex-1 max-w-xs">
              <Search size={13} className="text-[#74869D] shrink-0" />
              <input
                type="text"
                placeholder="Tìm theo bí danh, game profile..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-transparent text-[#EDF3FC] placeholder-[#74869D] outline-none text-xs"
              />
            </div>

            <div className="flex items-center gap-1 bg-[#101722] p-1 rounded-md border border-[#2A3647] text-xs">
              {['all', 'in_session', 'ready', 'cooldown'].map((st) => (
                <button
                  key={st}
                  onClick={() => setFilterStatus(st)}
                  className={`px-2.5 py-0.5 rounded font-medium transition-colors ${
                    filterStatus === st
                      ? 'bg-[#1878F5] text-white'
                      : 'text-[#74869D] hover:text-white'
                  }`}
                >
                  {st === 'all'
                    ? 'Tất cả'
                    : st === 'in_session'
                    ? 'Đang chạy'
                    : st === 'ready'
                    ? 'Sẵn sàng'
                    : 'Cooldown'}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Table */}
        <div className="flex-1 overflow-y-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-[#141C28] text-[#74869D] sticky top-0 border-b border-[#2A3647] z-10 font-bold uppercase tracking-wider text-[11px]">
              <tr>
                <th className="p-3">Bí danh tài khoản</th>
                <th className="p-3">Hồ sơ Game</th>
                <th className="p-3">Thiết bị gán</th>
                <th className="p-3">Tham chiếu Secret</th>
                <th className="p-3">Trạng thái</th>
                <th className="p-3">Vàng tích lũy</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#2A3647]">
              {filteredAccounts.map((acc) => {
                const isSelected = acc.id === selectedAccount?.id;
                return (
                  <tr
                    key={acc.id}
                    onClick={() => setSelectedAccountId(acc.id)}
                    className={`cursor-pointer transition-colors ${
                      isSelected
                        ? 'bg-[#17273D] border-l-4 border-l-[#1878F5]'
                        : 'hover:bg-[#16202F]'
                    }`}
                  >
                    <td className="p-3">
                      <div className="font-bold text-[#EDF3FC] text-[13px]">{acc.alias}</div>
                      <div className="text-[10px] text-[#74869D]">ID: {acc.id}</div>
                    </td>
                    <td className="p-3">
                      <div className="font-medium text-[#A9B8CC]">{acc.gameProfileId}</div>
                      <div className="text-[10px] text-[#7BA6D6]">Cấp độ {acc.level}</div>
                    </td>
                    <td className="p-3">
                      <span className="text-[#38BDF8] font-medium">{acc.assignedDevice}</span>
                    </td>
                    <td className="p-3 font-mono text-[11px] text-[#3BC38D]">
                      <div className="flex items-center gap-1.5">
                        <KeyRound size={12} className="shrink-0" />
                        <span className="truncate max-w-[170px]">{acc.secretRef}</span>
                      </div>
                    </td>
                    <td className="p-3">{renderStatusChip(acc.status)}</td>
                    <td className="p-3 font-mono text-[#FDE047] font-semibold">
                      {acc.goldFarmed.toLocaleString()} vàng
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Right: Account Contextual Drawer */}
      <div className="w-[360px] bg-[#141C28] flex flex-col overflow-y-auto p-4 space-y-4 shrink-0">
        {selectedAccount ? (
          <>
            <div className="border-b border-[#2A3647] pb-3">
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-[#74869D] uppercase font-mono">
                  {selectedAccount.id}
                </span>
                {renderStatusChip(selectedAccount.status)}
              </div>
              <h3 className="text-sm font-bold text-[#EDF3FC] mt-0.5">
                {selectedAccount.alias}
              </h3>
              <p className="text-[11px] text-[#74869D]">
                Hồ sơ: {selectedAccount.gameProfileId} (Level {selectedAccount.level})
              </p>
            </div>

            {/* Secret Reference Box */}
            <div className="p-3 rounded-lg bg-[#17202E] border border-[#2A3647] space-y-2">
              <div className="flex items-center gap-1.5 text-xs font-bold text-[#3BC38D]">
                <ShieldCheck size={14} />
                <span>Bảo vệ bởi Windows DPAPI</span>
              </div>
              <p className="text-[11px] text-[#A9B8CC] leading-relaxed">
                Mật khẩu và refresh token được mã hóa trực tiếp bởi hệ điều hành Windows qua khóa người dùng hiện tại.
              </p>
              <div className="p-2 rounded bg-[#101722] border border-[#2A3647] font-mono text-[10px] text-[#38BDF8] break-all">
                {selectedAccount.secretRef}
              </div>
            </div>

            {/* Session Stats */}
            <div className="p-3 rounded-lg bg-[#17202E] border border-[#2A3647] space-y-2 text-xs">
              <span className="font-bold text-[#EDF3FC] text-xs block">
                Thống kê Vận hành Phiên
              </span>

              <div className="space-y-1.5 text-[11px]">
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Thiết bị gán:</span>
                  <span className="font-semibold text-[#38BDF8]">
                    {selectedAccount.assignedDevice}
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Tài nguyên farm:</span>
                  <span className="font-mono text-[#FDE047] font-bold">
                    {selectedAccount.goldFarmed.toLocaleString()} Vàng
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Hoạt động gần nhất:</span>
                  <span className="text-[#EDF3FC]">{selectedAccount.lastActive}</span>
                </div>
              </div>

              <div className="pt-2 flex flex-col space-y-2">
                <button className="w-full py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center justify-center gap-1.5 transition-colors cursor-pointer">
                  <Play size={12} fill="currentColor" />
                  <span>Khởi chạy phiên cho tài khoản này</span>
                </button>
                <button className="w-full py-1.5 bg-[#192434] hover:bg-[#202E42] text-[#38BDF8] border border-[#2A3647] rounded-md text-xs font-semibold flex items-center justify-center gap-1.5 transition-colors cursor-pointer">
                  <RotateCw size={12} />
                  <span>Đổi thiết bị gán</span>
                </button>
              </div>
            </div>
          </>
        ) : (
          <div className="p-6 text-center text-[#74869D] text-xs">
            Chưa có tài khoản nào được chọn.
          </div>
        )}
      </div>

      {/* Add Account Modal */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4">
          <div className="w-[440px] bg-[#16202F] border border-[#2A3647] rounded-xl p-5 space-y-4 shadow-2xl animate-fadeIn">
            <div className="flex items-center justify-between border-b border-[#2A3647] pb-3">
              <h3 className="text-sm font-bold text-[#EDF3FC]">Thêm Tài khoản Mới</h3>
              <button
                onClick={() => setShowAddModal(false)}
                className="text-[#74869D] hover:text-white text-xs"
              >
                ✕
              </button>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Bí danh tài khoản</label>
                <input
                  type="text"
                  placeholder="Farmer_Alt_05"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Tên nhân vật / Game ID</label>
                <input
                  type="text"
                  placeholder="Hero_Warrior_Lv1"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">
                  Mật khẩu / Token (Lưu vào Windows Credential)
                </label>
                <input
                  type="password"
                  placeholder="••••••••••••"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
                />
                <p className="text-[10px] text-[#3BC38D] mt-1 flex items-center gap-1">
                  <ShieldCheck size={11} />
                  <span>Được mã hóa an toàn qua OS API, không lưu plaintext</span>
                </p>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[#2A3647]">
              <button
                onClick={() => setShowAddModal(false)}
                className="px-3 py-1.5 bg-[#101722] hover:bg-[#16202F] text-[#A9B8CC] rounded border border-[#2A3647] text-xs font-medium"
              >
                Hủy
              </button>
              <button
                onClick={() => setShowAddModal(false)}
                className="px-4 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded text-xs font-semibold"
              >
                Lưu vào Vault
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
