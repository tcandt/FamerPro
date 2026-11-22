import React, { useState, useEffect } from 'react';
import { AppHeader } from './AppHeader';
import { NavRail } from './NavRail';
import { StatusBar } from './StatusBar';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { useProjectStore } from '../../stores/projectStore';
import { api, isTauri } from '../../lib/api';

interface AppShellProps {
  currentView: string;
  onSelectView: (view: string) => void;
  children: React.ReactNode;
}

export const AppShell: React.FC<AppShellProps> = ({
  currentView,
  onSelectView,
  children,
}) => {
  const [isNavExpanded, setIsNavExpanded] = useState(true);
  const {
    workflowId,
    workflowName,
    nodes,
    edges,
    setIsRunning,
    setValidationReport,
    addRunEvent,
    clearRunEvents,
  } = useWorkflowStore();
  const { devices, selectedDeviceId, fetchDevices, fetchStatus } = useDeviceStore();
  const { fetchProjects, activeProjectId } = useProjectStore();

  useEffect(() => {
    fetchProjects(true);
    fetchDevices();
    fetchStatus();

    const interval = setInterval(() => {
      fetchDevices(false);
      fetchStatus();
    }, 5000);

    return () => clearInterval(interval);
  }, [fetchProjects, fetchDevices, fetchStatus]);

  const buildWorkflowDoc = () => ({
    schemaVersion: 1,
    workflowId,
    name: workflowName,
    status: 'draft',
    graph: {
      nodes: nodes.map((n) => ({
        id: n.id,
        type: n.data.nodeType,
        position: n.position,
        config: n.data.config || {},
      })),
      edges: edges.map((e) => ({
        id: e.id,
        source: e.source,
        sourcePort: e.sourceHandle || 'flow',
        target: e.target,
        targetPort: e.targetHandle || 'flow',
      })),
    },
  });

  const handleValidate = async () => {
    const doc = buildWorkflowDoc();
    const report = await api.validateWorkflow(JSON.stringify(doc));
    setValidationReport(report);
  };

  const handleRun = async () => {
    if (!isTauri()) {
      addRunEvent({
        id: `evt-web-err-${Date.now()}`,
        job_id: 'web-preview',
        node_id: 'system',
        event_type: 'verification_failed',
        attempt_no: 1,
        at: new Date().toISOString(),
        payload_json: JSON.stringify({
          error: 'Web Preview mode: Hãy chạy FarmerPro Desktop (.exe / tauri dev) để gọi RuntimeCoordinator.',
        }),
      });
      return;
    }

    setIsRunning(true);
    clearRunEvents();
    const doc = buildWorkflowDoc();
    const selectedDev = devices.find((d) => d.id === selectedDeviceId);
    const isMock = selectedDev ? !!selectedDev.is_mock : true;

    try {
      if (activeProjectId) {
        await api.studioSaveProjectWorkflow(
          activeProjectId,
          workflowId,
          workflowName,
          'Studio',
          JSON.stringify(doc)
        );
      }
      const resp = await api.executeWorkflow(
        JSON.stringify(doc),
        selectedDeviceId || 'testkit-sim-01',
        isMock
      );
      const events = await api.getAuditEvents(resp.job_id);
      events.forEach((evt) => addRunEvent(evt));
    } catch (err: any) {
      addRunEvent({
        id: `evt-err-${Date.now()}`,
        job_id: 'err-job',
        node_id: 'system',
        event_type: 'verification_failed',
        attempt_no: 1,
        at: new Date().toISOString(),
        payload_json: JSON.stringify({ error: err?.message || String(err) }),
      });
    } finally {
      setIsRunning(false);
    }
  };

  const handleStop = () => {
    setIsRunning(false);
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-[#101722] text-[#EDF3FC]">
      <AppHeader
        currentView={currentView}
        onSelectView={onSelectView}
        onValidate={handleValidate}
        onRun={handleRun}
        onStop={handleStop}
        onRefreshDevices={() => fetchDevices(true)}
      />
      <div className="flex flex-1 overflow-hidden relative">
        <NavRail
          currentView={currentView}
          onSelectView={onSelectView}
          isExpanded={isNavExpanded}
          onToggleExpand={() => setIsNavExpanded(!isNavExpanded)}
        />
        <main className="flex-1 flex overflow-hidden relative bg-[#101722]">
          {children}
        </main>
      </div>
      <StatusBar />
    </div>
  );
};
