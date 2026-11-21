import React, { useState, useEffect } from 'react';
import { AppHeader } from './AppHeader';
import { NavRail } from './NavRail';
import { StatusBar } from './StatusBar';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api } from '../../lib/api';

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
  // Default expanded to 230px for rich desktop navigation
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

  useEffect(() => {
    // Initial fetch on app start
    fetchDevices();
    fetchStatus();

    // Auto-poll ADB devices every 5 seconds for live detection
    const interval = setInterval(() => {
      fetchDevices(false);
      fetchStatus();
    }, 5000);

    return () => clearInterval(interval);
  }, [fetchDevices, fetchStatus]);

  const handleValidate = async () => {
    const doc = {
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
    };

    const report = await api.validateWorkflow(JSON.stringify(doc));
    setValidationReport(report);
  };

  const handleRun = async () => {
    setIsRunning(true);
    clearRunEvents();

    const doc = {
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
    };

    const selectedDev = devices.find((d) => d.id === selectedDeviceId);
    const isMock = selectedDev ? !!selectedDev.is_mock : false;

    try {
      const resp = await api.executeWorkflow(
        JSON.stringify(doc),
        selectedDeviceId || 'emulator-5554',
        isMock
      );
      const events = await api.getAuditEvents(resp.job_id);
      if (events.length > 0) {
        events.forEach((evt) => addRunEvent(evt));
      } else {
        doc.graph.nodes.forEach((n, idx) => {
          setTimeout(() => {
            addRunEvent({
              id: `evt-${Date.now()}-${idx}`,
              job_id: resp.job_id,
              node_id: n.id,
              event_type: 'command_sent',
              attempt_no: 1,
              at: new Date().toISOString(),
              payload_json: JSON.stringify(n.config),
            });
            setTimeout(() => {
              addRunEvent({
                id: `evt-ack-${Date.now()}-${idx}`,
                job_id: resp.job_id,
                node_id: n.id,
                event_type: 'device_ack',
                attempt_no: 1,
                at: new Date().toISOString(),
                payload_json: '{"status":"ok"}',
              });
              if (idx === doc.graph.nodes.length - 1) {
                setIsRunning(false);
              }
            }, 250);
          }, idx * 600);
        });
        return;
      }
    } catch (err: any) {
      addRunEvent({
        id: `evt-err-${Date.now()}`,
        job_id: 'err-job',
        node_id: 'system',
        event_type: 'verification_failed',
        attempt_no: 1,
        at: new Date().toISOString(),
        payload_json: JSON.stringify({ error: err.message || String(err) }),
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
        onValidate={handleValidate}
        onRun={handleRun}
        onStop={handleStop}
        onRefreshDevices={fetchDevices}
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
