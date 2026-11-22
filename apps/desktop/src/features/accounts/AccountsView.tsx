import React, { useMemo, useState } from 'react';
import {
  AlertCircle,
  Clock,
  KeyRound,
  Plus,
  RefreshCw,
  Search,
  ShieldCheck,
  X,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { api } from '../../lib/api';
import type { ProjectAccountRecord } from '../../types';

export const AccountsView: React.FC = () => {
  const { activeProjectId, activeProjectDetail, refreshActiveProject } = useProjectStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [alias, setAlias] = useState('');
  const [gameProfileId, setGameProfileId] = useState('');
  const [secretRef, setSecretRef] = useState('');
  const [stateValue, setStateValue] = useState('ready');

  const accounts: ProjectAccountRecord[] = activeProjectDetail?.accounts || [];

  const filteredAccounts = useMemo(() => {
    return accounts.filter((acc) => {
      if (filterStatus !== 'all' && acc.state !== filterStatus) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        return (
          acc.alias.toLowerCase().includes(q) ||
          (acc.gameProfileId || '').toLowerCase().includes(q) ||
          (acc.secretRef || '').toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [accounts, filterStatus, searchQuery]);

  const selectedAccount =
    filteredAccounts.find((a) => a.id === selectedAccountId) || filteredAccounts[0] || null;

  const handleSaveAccount = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeProjectId || !alias.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await api.studioUpsertProjectAccount({
        projectId: activeProjectId,
        alias: alias.trim(),
        gameProfileId: gameProfileId.trim() || null,
        secretRef:
          secretRef.trim() ||
          `dpapi://${activeProjectDetail?.project.slug || 'project'}/${alias.trim()}`,
        state: stateValue,
      });
      await refreshActiveProject();
      setShowModal(false);
      setAlias('');
      setGameProfileId('');
      setSecretRef('');
      setStateValue('ready');
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setSaving(false);
    }
  };

  const renderStateChip = (st: string) => {
    switch (st) {
      case 'in_session':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#173355] text-[#38BDF8] border border-[#38BDF8]/40 w-fit inline-flex items-center gap-1">
            <span>in_session</span>
          </span>
        );
      case 'ready':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40 w-fit inline-flex items-center gap-1">
            <span>ready</span>
          </span>
        );
      case 'cooldown':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#3B2D17] text-[#E7B15B] border border-[#E7B15B]/40 w-fit inline-flex items-center gap-1">
            <Clock size={10} />
            <span>cooldown</span>
          </span>
        );
      default:
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#381E24] text-[#F27883] border border-[#F27883]/40 w-fit inline-flex items-center gap-1">
            <AlertCircle size={10} />
            <span>{st}</span>
          </span>
        );
    }
  };

  if (!activeProjectId || !activeProjectDetail) {
    return (
      <div className="flex-1 flex items-center justify-center bg-[#101722] p-6">
        <div className="fp-panel p-6 max-w-md text-center space-y-2">
          <KeyRound size={26} className="mx-auto text-[#4B9CFF]" />
          <div className="text-sm font-bold text-[#EDF3FC]">Chưa chọn Game Project</div>
          <p className="text-xs text-[#74869D]">
            Chọn một Game Project từ Header để xem và quản lý tài khoản trong bảng{' '}
            <code>project_accounts</code>.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex overflow-hidden bg-[#101722] text-[#EDF3FC]">
      <div className="flex-1 flex flex-col border-r border-[#2A3647] overflow-hidden">
        <div className="p-4 border-b border-[#2A3647] space-y-3 bg-[#141C28]">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-sm font-bold text-[#EDF3FC]">
                  Account Vault — {activeProjectDetail.project.name}
                </h1>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
                  {accounts.length} tài khoản (SQLite)
                </span>
              </div>
              <p className="text-xs text-[#74869D] mt-0.5">
                Chỉ lưu tham chiếu bí mật (<code>secret_ref</code>) theo từng dự án; mật khẩu được
                bảo vệ riêng qua Windows DPAPI CurrentUser.
              </p>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={() => refreshActiveProject()}
                className="px-2.5 py-1.5 rounded bg-[#161E29] border border-[#2A3647] text-xs text-[#A9B8CC] flex items-center gap-1 cursor-pointer"
              >
                <RefreshCw size={12} />
                <span>Làm mới</span>
              </button>
              <button
                onClick={() => setShowModal(true)}
                className="px-3 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 cursor-pointer"
              >
                <Plus size={13} />
                <span>Thêm tài khoản</span>
              </button>
            </div>
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs flex-1 max-w-xs">
              <Search size={13} className="text-[#74869D] shrink-0" />
              <input
                type="text"
                placeholder="Tìm alias, profile, secretRef..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-transparent text-[#EDF3FC] placeholder-[#74869D] outline-none text-xs"
              />
            </div>

            <div className="flex items-center gap-1 bg-[#101722] p-1 rounded-md border border-[#2A3647] text-xs">
              {['all', 'ready', 'in_session', 'cooldown'].map((st) => (
                <button
                  key={st}
                  onClick={() => setFilterStatus(st)}
                  className={`px-2.5 py-0.5 rounded font-medium cursor-pointer ${
                    filterStatus === st
                      ? 'bg-[#1878F5] text-white'
                      : 'text-[#74869D] hover:text-white'
                  }`}
                >
                  {st}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {filteredAccounts.length === 0 ? (
            <div className="p-6 rounded-lg bg-[#16202F] border border-dashed border-[#2A3647] text-center text-xs text-[#74869D]">
              Project này chưa có tài khoản nào trong <code>project_accounts</code>.
            </div>
          ) : (
            <table className="w-full text-left text-xs border-collapse">
              <thead className="bg-[#141C28] text-[#74869D] border-b border-[#2A3647] uppercase text-[11px]">
                <tr>
                  <th className="p-3">Alias</th>
                  <th className="p-3">Game Profile</th>
                  <th className="p-3">Secret Ref</th>
                  <th className="p-3">Trạng thái</th>
                  <th className="p-3">Cập nhật</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2A3647]">
                {filteredAccounts.map((acc) => (
                  <tr
                    key={acc.id}
                    onClick={() => setSelectedAccountId(acc.id)}
                    className={`cursor-pointer ${
                      acc.id === selectedAccount?.id ? 'bg-[#17273D]' : 'hover:bg-[#16202F]'
                    }`}
                  >
                    <td className="p-3 font-semibold text-[#EDF3FC]">{acc.alias}</td>
                    <td className="p-3 text-[#A9B8CC]">{acc.gameProfileId || '—'}</td>
                    <td className="p-3 font-mono text-[11px] text-[#3BC38D]">
                      {acc.secretRef || '—'}
                    </td>
                    <td className="p-3">{renderStateChip(acc.state)}</td>
                    <td className="p-3 font-mono text-[11px] text-[#74869D]">
                      {new Date(acc.updatedAt).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>

      <div className="w-[320px] bg-[#141C28] p-4 space-y-4 shrink-0 overflow-y-auto">
        {selectedAccount ? (
          <>
            <div className="border-b border-[#2A3647] pb-3 space-y-1">
              <div className="font-mono text-[10px] text-[#74869D]">{selectedAccount.id}</div>
              <div className="text-sm font-bold text-[#EDF3FC]">{selectedAccount.alias}</div>
              {renderStateChip(selectedAccount.state)}
            </div>
            <div className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] space-y-1.5 text-xs">
              <div className="flex items-center gap-1.5 text-[#3BC38D] font-bold">
                <ShieldCheck size={14} />
                <span>Secret Reference Isolation</span>
              </div>
              <div className="font-mono text-[11px] text-[#A9B8CC] break-all">
                {selectedAccount.secretRef || 'Chưa gán secretRef'}
              </div>
            </div>
          </>
        ) : (
          <div className="text-xs text-[#74869D]">Chọn một tài khoản để xem chi tiết.</div>
        )}
      </div>

      {showModal && (
        <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
          <form
            onSubmit={handleSaveAccount}
            className="w-full max-w-md rounded-xl bg-[#16202F] border border-[#2A3647] p-5 space-y-3 text-xs"
          >
            <div className="flex items-center justify-between border-b border-[#2A3647] pb-2.5">
              <h3 className="text-sm font-bold text-[#EDF3FC]">Thêm tài khoản vào Project</h3>
              <button
                type="button"
                onClick={() => setShowModal(false)}
                className="text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
              >
                <X size={15} />
              </button>
            </div>
            {error && (
              <div className="p-2 rounded bg-[#381E24] text-[#F27883] border border-[#F27883]/40">
                {error}
              </div>
            )}
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold">Alias</label>
              <input
                type="text"
                required
                value={alias}
                onChange={(e) => setAlias(e.target.value)}
                placeholder="acc_worker_01"
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold">Game Profile ID</label>
              <input
                type="text"
                value={gameProfileId}
                onChange={(e) => setGameProfileId(e.target.value)}
                placeholder="profile-server-01"
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold">
                Secret Ref (DPAPI URI)
              </label>
              <input
                type="text"
                value={secretRef}
                onChange={(e) => setSecretRef(e.target.value)}
                placeholder={`dpapi://${activeProjectDetail.project.slug}/acc_01`}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold">Trạng thái</label>
              <select
                value={stateValue}
                onChange={(e) => setStateValue(e.target.value)}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
              >
                <option value="ready">ready</option>
                <option value="in_session">in_session</option>
                <option value="cooldown">cooldown</option>
                <option value="expired">expired</option>
              </select>
            </div>
            <div className="flex justify-end gap-2 pt-2 border-t border-[#2A3647]">
              <button
                type="button"
                onClick={() => setShowModal(false)}
                className="px-3 py-1.5 rounded bg-[#101722] border border-[#2A3647] text-[#A9B8CC]"
              >
                Hủy
              </button>
              <button
                type="submit"
                disabled={saving}
                className="px-4 py-1.5 rounded bg-[#1878F5] text-white font-semibold cursor-pointer"
              >
                {saving ? 'Đang lưu...' : 'Lưu vào SQLite'}
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};
