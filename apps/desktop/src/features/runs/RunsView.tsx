import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Activity,
  AlertTriangle,
  Bookmark,
  CheckCircle2,
  Copy,
  Lock,
  Pause,
  Play,
  RefreshCw,
  RotateCcw,
  ShieldAlert,
  Smartphone,
  Square,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api } from '../../lib/api';
import type {
  ActiveDeviceLeaseView,
  JobResumeDetail,
  ProjectJobRecord,
  StepEvent,
} from '../../types';

export const RunsView: React.FC = () => {
  const { activeProjectId, activeProjectDetail } = useProjectStore();
  const { runEvents, isRunning, setRunEvents, setActiveJobState, clearRunEvents } =
    useWorkflowStore();
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();

  const [jobs, setJobs] = useState<ProjectJobRecord[]>([]);
  const [leases, setLeases] = useState<ActiveDeviceLeaseView[]>([]);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [jobDetail, setJobDetail] = useState<JobResumeDetail | null>(null);
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [statusMsg, setStatusMsg] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null);

  const workflows = activeProjectDetail?.workflows || [];

  useEffect(() => {
    if (workflows.length > 0 && !workflows.some((w) => w.id === selectedWorkflowId)) {
      setSelectedWorkflowId(workflows[0].id);
    }
  }, [workflows, selectedWorkflowId]);

  const loadJobsAndLeases = useCallback(async () => {
    if (!activeProjectId) {
      setJobs([]);
      setLeases([]);
      setJobDetail(null);
      return;
    }
    setLoading(true);
    try {
      const [jobList, leaseList] = await Promise.all([
        api.studioListProjectJobs(activeProjectId),
        api.studioListActiveDeviceLeases(),
      ]);
      setJobs(jobList);
      setLeases(leaseList);
      const nextJobId =
        selectedJobId && jobList.some((j) => j.id === selectedJobId)
          ? selectedJobId
          : jobList[0]?.id || null;
      setSelectedJobId(nextJobId);
      if (nextJobId) {
        const detail = await api.studioGetJobResumeDetail(nextJobId);
        setJobDetail(detail);
      } else {
        setJobDetail(null);
      }
    } catch (err: any) {
      setStatusMsg({ kind: 'err', text: `Không tải được danh sách Jobs: ${err?.message || err}` });
    } finally {
      setLoading(false);
    }
  }, [activeProjectId, selectedJobId]);

  useEffect(() => {
    void loadJobsAndLeases();
  }, [activeProjectId]);

  const handleSelectJob = async (jobId: string) => {
    setSelectedJobId(jobId);
    setSelectedEventId(null);
    try {
      const detail = await api.studioGetJobResumeDetail(jobId);
      setJobDetail(detail);
    } catch (err: any) {
      setStatusMsg({ kind: 'err', text: `Không tải được chi tiết Job: ${err?.message || err}` });
    }
  };

  const handleRunTestKit = async () => {
    if (!activeProjectId || !selectedWorkflowId) return;
    const targetDevice = devices.find((d) => d.id === selectedDeviceId)?.id || 'testkit-emulator';
    setLoading(true);
    setStatusMsg(null);
    try {
      const summary = await api.studioRunProjectWorkflow({
        projectId: activeProjectId,
        workflowId: selectedWorkflowId,
        deviceId: targetDevice,
        priority: 1,
        leaseTtlSecs: 120,
        workflowTimeoutMs: 60000,
      });
      setSelectedJobId(summary.jobId);
      const detail = await api.studioGetJobResumeDetail(summary.jobId);
      setJobDetail(detail);
      setActiveJobState(
        summary.jobId,
        detail,
        summary,
        `Job ${summary.jobId} (${summary.adapterKind} · ${summary.state})`
      );
      if (detail?.stepEvents) {
        setRunEvents(detail.stepEvents);
      }
      const [jobList, leaseList] = await Promise.all([
        api.studioListProjectJobs(activeProjectId),
        api.studioListActiveDeviceLeases(),
      ]);
      setJobs(jobList);
      setLeases(leaseList);
      setStatusMsg({
        kind: 'ok',
        text: `Đã chạy Job ${summary.jobId} qua ${summary.adapterKind} (${summary.state}, ${summary.eventsRecorded} events).`,
      });
    } catch (err: any) {
      setStatusMsg({ kind: 'err', text: `Lỗi thực thi TestKit: ${err?.message || err}` });
    } finally {
      setLoading(false);
    }
  };

  const handleJobAction = async (
    action: 'pause' | 'resume' | 'suspend' | 'cancel' | 'resume_cp',
    jobId: string
  ) => {
    setLoading(true);
    setStatusMsg(null);
    try {
      if (action === 'pause') await api.studioPauseJob(jobId);
      else if (action === 'resume') await api.studioResumeJob(jobId);
      else if (action === 'suspend') await api.studioSuspendJobToCheckpoint(jobId);
      else if (action === 'cancel') await api.studioCancelJob(jobId);
      else if (action === 'resume_cp') {
        const summary = await api.studioResumeJobFromCheckpoint(jobId, 120, 60000);
        const detail = await api.studioGetJobResumeDetail(jobId);
        setActiveJobState(
          jobId,
          detail,
          summary,
          `Checkpoint resume ${jobId}: ${summary.state}`
        );
        setStatusMsg({
          kind: 'ok',
          text: `Đã khôi phục từ Checkpoint cho Job ${jobId}: ${summary.state} (từ node ${
            summary.resumedFromNodeId || 'n/a'
          }).`,
        });
      }
      await loadJobsAndLeases();
    } catch (err: any) {
      setStatusMsg({ kind: 'err', text: `Thao tác Job thất bại: ${err?.message || err}` });
    } finally {
      setLoading(false);
    }
  };

  const displayEvents: StepEvent[] = useMemo(() => {
    if (jobDetail?.stepEvents && jobDetail.stepEvents.length > 0) {
      return jobDetail.stepEvents;
    }
    return runEvents;
  }, [jobDetail?.stepEvents, runEvents]);

  const activeEvent =
    displayEvents.find((e) => e.id === selectedEventId) ||
    displayEvents[displayEvents.length - 1] ||
    null;

  const parsedPayload = useMemo(() => {
    if (!activeEvent?.payload_json) return null;
    try {
      return JSON.parse(activeEvent.payload_json);
    } catch {
      return { raw: activeEvent.payload_json };
    }
  }, [activeEvent?.payload_json]);

  const selectedJob = jobs.find((j) => j.id === selectedJobId) || null;
  const checkpoint = jobDetail?.resumeContext?.latestCheckpoint || null;
  const isTestkitJob = (selectedJob?.deviceId || '').toLowerCase().includes('testkit');

  return (
    <div className="flex-1 flex flex-col p-5 space-y-4 overflow-hidden bg-[#101722] text-[#EDF3FC]">
      {/* Header */}
      <div className="fp-panel p-4 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-3">
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-[#173355] text-[#38BDF8] border border-[#2A3647]">
              TEST &amp; DEBUG LAB
            </span>
            <span className={isRunning ? 'fp-badge-testkit' : 'fp-badge-offline'}>
              {isRunning ? 'ĐANG CHẠY WORKFLOW' : 'STANDBY'}
            </span>
            {activeProjectDetail && (
              <span className="text-xs font-mono text-[#A9B8CC]">
                {activeProjectDetail.project.slug} · {jobs.length} jobs · {leases.length} active
                leases
              </span>
            )}
          </div>
          <h1 className="text-lg font-bold text-[#EDF3FC] mt-1">
            Kiểm thử, Giám sát Step Trace &amp; Khôi phục Checkpoint
          </h1>
          <p className="text-xs text-[#74869D]">
            Dữ liệu lấy trực tiếp từ <code>project_jobs</code>, <code>step_events</code>,{' '}
            <code>checkpoints</code> và <code>device_leases</code> trong SQLite WAL.
          </p>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <div className="flex items-center gap-1.5 bg-[#161E29] border border-[#2A3647] rounded-md px-2.5 py-1.5 text-xs">
            <Smartphone size={13} className="text-[#4B9CFF]" />
            <select
              value={selectedDeviceId || 'testkit-emulator'}
              onChange={(e) => selectDevice(e.target.value)}
              className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs font-semibold"
            >
              <option value="testkit-emulator" className="bg-[#161E29]">
                testkit-emulator (TestKit Sandbox)
              </option>
              {devices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[#161E29]">
                  {d.alias} ({d.adb_serial})
                </option>
              ))}
            </select>
          </div>

          {workflows.length > 0 && (
            <select
              value={selectedWorkflowId}
              onChange={(e) => setSelectedWorkflowId(e.target.value)}
              className="bg-[#161E29] border border-[#2A3647] rounded-md px-2.5 py-1.5 text-xs text-[#EDF3FC] outline-none"
            >
              {workflows.map((wf) => (
                <option key={wf.id} value={wf.id}>
                  {wf.name}
                </option>
              ))}
            </select>
          )}

          <button
            onClick={handleRunTestKit}
            disabled={loading || !activeProjectId || !selectedWorkflowId}
            className="px-3 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] disabled:opacity-50 text-white rounded-md text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
          >
            <Play size={12} />
            <span>Chạy TestKit</span>
          </button>

          <button
            onClick={() => loadJobsAndLeases()}
            disabled={loading}
            className="px-2.5 py-1.5 bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] text-[#A9B8CC] rounded-md text-xs flex items-center gap-1 cursor-pointer"
          >
            <RefreshCw size={12} />
            <span>Làm mới</span>
          </button>

          <button
            onClick={() => {
              if (displayEvents.length > 0) {
                navigator.clipboard?.writeText(JSON.stringify(displayEvents, null, 2));
              }
            }}
            className="px-2.5 py-1.5 bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] text-[#EDF3FC] rounded-md text-xs flex items-center gap-1 cursor-pointer"
          >
            <Copy size={12} />
            <span>JSON</span>
          </button>
        </div>
      </div>

      {statusMsg && (
        <div
          className={`px-3.5 py-2 rounded-lg border text-xs flex items-center gap-2 ${
            statusMsg.kind === 'ok'
              ? 'bg-[#133528] border-[#3BC38D]/40 text-[#3BC38D]'
              : 'bg-[#381E24] border-[#F27883]/40 text-[#F27883]'
          }`}
        >
          {statusMsg.kind === 'ok' ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
          <span>{statusMsg.text}</span>
        </div>
      )}

      {/* Main 12-col Split: 4 cols Jobs & Leases | 4 cols Step Trace & Checkpoints | 4 cols Deep Inspector */}
      <div className="grid grid-cols-12 gap-4 flex-1 min-h-0">
        {/* Column 1: Device Leases & Project Jobs */}
        <div className="col-span-12 lg:col-span-4 flex flex-col gap-4 min-h-0 overflow-hidden">
          <div className="fp-panel p-3.5 space-y-2 shrink-0">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 text-xs font-bold text-[#EDF3FC]">
                <Lock size={13} className="text-[#4B9CFF]" />
                <span>Device Leases đang giữ ({leases.length})</span>
              </div>
              <span className="text-[10px] font-mono text-[#74869D]">Fencing Token</span>
            </div>
            {leases.length === 0 ? (
              <p className="text-[11px] text-[#74869D]">
                0 lease đang hoạt động (không có thiết bị nào bị khóa bởi Coordinator).
              </p>
            ) : (
              <div className="space-y-1.5 max-h-28 overflow-y-auto">
                {leases.map((l) => (
                  <div
                    key={l.deviceId}
                    className="p-2 rounded bg-[#101722] border border-[#2A3647] text-[11px] font-mono flex items-center justify-between"
                  >
                    <span className="text-[#38BDF8] truncate">{l.deviceId}</span>
                    <span className="text-[#A9B8CC]">
                      gen#{l.generation} · {l.jobId.slice(0, 10)}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="flex-1 fp-panel p-3.5 flex flex-col min-h-0 overflow-hidden">
            <div className="flex items-center justify-between pb-2 border-b border-[#2A3647]">
              <div className="flex items-center gap-1.5 text-xs font-bold text-[#EDF3FC]">
                <Activity size={14} className="text-[#3BC38D]" />
                <span>Lịch sử Jobs ({jobs.length})</span>
              </div>
              <button
                onClick={clearRunEvents}
                className="text-[11px] text-[#74869D] hover:text-[#F27883] cursor-pointer"
              >
                Xóa buffer
              </button>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 mt-2.5 pr-1">
              {jobs.length === 0 ? (
                <div className="p-4 rounded bg-[#101722] border border-dashed border-[#2A3647] text-center text-[11px] text-[#74869D]">
                  Chưa có Job nào trong SQLite cho dự án này. Nhấn &quot;Chạy TestKit&quot; để tạo
                  phiên chạy thật.
                </div>
              ) : (
                jobs.map((job) => {
                  const active = job.id === selectedJobId;
                  return (
                    <div
                      key={job.id}
                      onClick={() => handleSelectJob(job.id)}
                      className={`p-2.5 rounded-lg border text-xs cursor-pointer transition-colors space-y-1 ${
                        active
                          ? 'bg-[#172E49] border-[#38BDF8]'
                          : 'bg-[#101722] border-[#2A3647] hover:border-[#3D4F67]'
                      }`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-mono font-bold text-[#EDF3FC] truncate">
                          {job.id}
                        </span>
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[#16202F] border border-[#2A3647] text-[#38BDF8]">
                          {job.state}
                        </span>
                      </div>
                      <div className="text-[11px] text-[#74869D] flex items-center justify-between font-mono">
                        <span>{job.deviceId}</span>
                        <span>gen#{job.fencingGeneration}</span>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>
        </div>

        {/* Column 2: Step Trace & Checkpoint Recovery */}
        <div className="col-span-12 lg:col-span-4 flex flex-col gap-4 min-h-0 overflow-hidden">
          <div className="flex-1 fp-panel p-3.5 flex flex-col min-h-0 overflow-hidden">
            <div className="flex items-center justify-between pb-2 border-b border-[#2A3647]">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC]">Dòng sự kiện (Step Trace)</h3>
                <span className="text-[11px] text-[#74869D]">
                  {selectedJob
                    ? `Job ${selectedJob.id} (${isTestkitJob ? 'TESTKIT' : 'DEVICE'})`
                    : 'Buffer hiện tại'}
                </span>
              </div>
              <span className="font-mono text-xs text-[#38BDF8] font-semibold">
                {displayEvents.length} events
              </span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 mt-2.5 pr-1">
              {displayEvents.length === 0 ? (
                <div className="p-4 rounded bg-[#101722] border border-dashed border-[#2A3647] text-center text-[11px] text-[#74869D]">
                  Chưa có Step Event nào. Chọn một Job hoặc nhấn Chạy TestKit.
                </div>
              ) : (
                displayEvents.map((ev, idx) => {
                  const isSelected = ev.id === activeEvent?.id;
                  return (
                    <div
                      key={ev.id}
                      onClick={() => setSelectedEventId(ev.id)}
                      className={`p-2.5 rounded-lg border text-xs cursor-pointer transition-colors flex items-center justify-between gap-2 ${
                        isSelected
                          ? 'bg-[#172E49] border-[#38BDF8]'
                          : 'bg-[#101722] border-[#2A3647] hover:border-[#3D4F67]'
                      }`}
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="font-mono text-[10px] text-[#5A6D85]">#{idx + 1}</span>
                        <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-bold shrink-0">
                          {ev.node_id}
                        </span>
                        <div className="min-w-0">
                          <div className="font-semibold text-[#EDF3FC] truncate">
                            {ev.event_type}
                          </div>
                          <div className="text-[10px] text-[#74869D] font-mono truncate max-w-[180px]">
                            {ev.payload_json}
                          </div>
                        </div>
                      </div>
                      <span className="text-[10px] text-[#5A6D85] font-mono shrink-0">
                        {new Date(ev.at).toLocaleTimeString()}
                      </span>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Checkpoint & Job Controls */}
          <div className="fp-panel p-3.5 space-y-2.5 shrink-0">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 text-xs font-bold text-[#EDF3FC]">
                <Bookmark size={14} className="text-[#FDE047]" />
                <span>Checkpoint &amp; Điều khiển Coordinator</span>
              </div>
              {selectedJob && (
                <span className="font-mono text-[10px] text-[#A9B8CC]">{selectedJob.state}</span>
              )}
            </div>

            {checkpoint ? (
              <div className="p-2.5 rounded bg-[#101722] border border-[#2A3647] text-[11px] space-y-1">
                <div className="flex items-center justify-between font-mono">
                  <span className="text-[#FDE047] font-bold">Node: {checkpoint.nodeId}</span>
                  <span className="text-[#74869D]">
                    {new Date(checkpoint.createdAt).toLocaleTimeString()}
                  </span>
                </div>
                <div className="font-mono text-[10px] text-[#A9B8CC] truncate">
                  ID: {checkpoint.id}
                </div>
              </div>
            ) : (
              <p className="text-[11px] text-[#74869D]">
                Chưa có checkpoint được ghi nhận cho Job đang chọn.
              </p>
            )}

            {selectedJob && (
              <div className="flex items-center gap-1.5 flex-wrap pt-1">
                <button
                  onClick={() => handleJobAction('resume_cp', selectedJob.id)}
                  disabled={loading}
                  className="px-2.5 py-1.5 rounded bg-[#192434] hover:bg-[#1F334E] border border-[#2A3647] text-[#38BDF8] text-[11px] font-semibold flex items-center gap-1 cursor-pointer"
                >
                  <RotateCcw size={11} />
                  <span>Resume Checkpoint</span>
                </button>
                <button
                  onClick={() => handleJobAction('pause', selectedJob.id)}
                  disabled={loading}
                  className="px-2 py-1.5 rounded bg-[#161E29] border border-[#2A3647] text-[#A9B8CC] text-[11px] flex items-center gap-1 cursor-pointer"
                >
                  <Pause size={11} />
                  <span>Pause</span>
                </button>
                <button
                  onClick={() => handleJobAction('suspend', selectedJob.id)}
                  disabled={loading}
                  className="px-2 py-1.5 rounded bg-[#161E29] border border-[#2A3647] text-[#E7B15B] text-[11px] cursor-pointer"
                >
                  Suspend CP
                </button>
                <button
                  onClick={() => handleJobAction('cancel', selectedJob.id)}
                  disabled={loading}
                  className="px-2 py-1.5 rounded bg-[#161E29] border border-[#2A3647] text-[#F27883] text-[11px] flex items-center gap-1 cursor-pointer"
                >
                  <Square size={10} />
                  <span>Cancel</span>
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Column 3: Step Payload & Conditional Safe Tap Visualizer */}
        <div className="col-span-12 lg:col-span-4 fp-panel p-4 flex flex-col min-h-0 overflow-y-auto space-y-4">
          {!activeEvent ? (
            <div className="flex-1 flex flex-col items-center justify-center text-center p-6 space-y-2 text-[#74869D]">
              <ShieldAlert size={26} />
              <div className="text-xs font-semibold text-[#EDF3FC]">Chưa chọn Step Event</div>
              <p className="text-[11px]">
                Nhấp một dòng trong Step Trace để kiểm tra JSON payload, Safe Tap bounds (nếu có)
                hoặc lý do chặn thiết bị.
              </p>
            </div>
          ) : (
            <>
              <div className="border-b border-[#2A3647] pb-3">
                <div className="flex items-center justify-between">
                  <span className="text-[10px] text-[#74869D] font-mono uppercase">
                    STEP NODE: {activeEvent.node_id}
                  </span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#173355] text-[#38BDF8] font-mono font-bold">
                    attempt #{activeEvent.attempt_no}
                  </span>
                </div>
                <h3 className="text-sm font-bold text-[#EDF3FC] mt-1">{activeEvent.event_type}</h3>
                <span className="text-[11px] text-[#74869D] font-mono">
                  {new Date(activeEvent.at).toLocaleString()} · Job {activeEvent.job_id}
                </span>
              </div>

              {parsedPayload &&
                (parsedPayload.sampled_point || parsedPayload.bounds || parsedPayload.reason) && (
                  <div className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] space-y-2 text-xs">
                    {parsedPayload.reason && (
                      <div className="text-[#E7B15B] font-mono text-[11px]">
                        Block/Reason: {String(parsedPayload.reason)}
                      </div>
                    )}
                    {parsedPayload.sampled_point && (
                      <div className="text-[#38BDF8] font-mono text-[11px]">
                        Safe Tap Sampled: ({String(parsedPayload.sampled_point[0])},{' '}
                        {String(parsedPayload.sampled_point[1])})
                      </div>
                    )}
                    {parsedPayload.bounds && (
                      <div className="text-[#A9B8CC] font-mono text-[11px]">
                        Bounds: {JSON.stringify(parsedPayload.bounds)}
                      </div>
                    )}
                  </div>
                )}

              <div className="space-y-1.5 flex-1">
                <span className="text-xs font-bold text-[#EDF3FC] block">
                  Dữ liệu JSON Payload (SQLite step_events)
                </span>
                <div className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#A9B8CC] overflow-x-auto leading-relaxed">
                  <pre>{JSON.stringify(parsedPayload ?? {}, null, 2)}</pre>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
