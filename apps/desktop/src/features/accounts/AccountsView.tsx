import React from 'react';
import { KeyRound, Plus } from 'lucide-react';

export const AccountsView: React.FC = () => {
  const accounts = [
    { id: 'acc-01', alias: 'Farmer_Main_01', game: 'Example RPG', assignedDevice: 'Pixel 6 — Máy 01', secretRef: 'CRED_MGR://farmerpro/acc01' },
    { id: 'acc-02', alias: 'Farmer_Alt_02', game: 'Example RPG', assignedDevice: 'Galaxy S22 — Máy 02', secretRef: 'CRED_MGR://farmerpro/acc02' },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Quản lý Tài khoản (Accounts)</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Bí danh tài khoản và tham chiếu thông tin đăng nhập từ Windows Credential Manager. Tuyệt đối không lưu mật khẩu trần trong JSON.
          </p>
        </div>
        <button className="px-3.5 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded flex items-center gap-1.5 shadow-sm">
          <Plus size={14} />
          <span>Thêm tài khoản</span>
        </button>
      </div>

      <div className="rounded-[9px] bg-[#192230] border border-[#2A3647] overflow-hidden">
        <table className="w-full text-left text-xs">
          <thead className="bg-[#161E29] text-[#74869D] border-b border-[#2A3647]">
            <tr>
              <th className="p-3 font-medium">Bí danh</th>
              <th className="p-3 font-medium">Game Profile</th>
              <th className="p-3 font-medium">Thiết bị gán</th>
              <th className="p-3 font-medium">Kho bảo mật (Secret Ref)</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#2A3647]">
            {accounts.map((acc) => (
              <tr key={acc.id} className="hover:bg-[#202B3B]">
                <td className="p-3 font-medium text-[#EDF3FC]">{acc.alias}</td>
                <td className="p-3 text-[#A9B8CC]">{acc.game}</td>
                <td className="p-3 text-[#7BA6D6]">{acc.assignedDevice}</td>
                <td className="p-3 font-mono text-[#3BC38D] flex items-center gap-1.5">
                  <KeyRound size={12} />
                  <span>{acc.secretRef}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
