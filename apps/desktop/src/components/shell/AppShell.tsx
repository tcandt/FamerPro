import React, { useState } from 'react';
import { AppTitlebar } from './AppTitlebar';
import { AppCommandBar } from './AppCommandBar';
import { NavRail } from './NavRail';
import { StatusBar } from './StatusBar';
import { useWorkflowStore } from '../../stores/workflowStore';

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
  const [isNavExpanded, setIsNavExpanded] = useState(false);
  const { nodes, setIsRunning, setValidationReport, addRunEvent, clearRunEvents } = useWorkflowStore();

  const handleValidate = () => {
    const hasStart = nodes.some((n) => n.data.nodeType === 'flow.start');
    const hasEnd = nodes.some((n) => n.data.nodeType === 'flow.end');

    const errors: string[] = [];
    if (!hasStart) errors.push("Thiếu node 'flow.start' (Bắt đầu)");
    if (!hasEnd) errors.push("Thiếu node 'flow.end' (Kết thúc)");

    const is_valid = errors.length === 0;
    setValidationReport({
      is_valid,
      errors,
      warnings: [],
    });
  };

  const handleRun = () => {
    setIsRunning(true);
    clearRunEvents();

    const sequence = ['n-start', 'n-launch', 'n-capture', 'n-find', 'n-tap', 'n-verify', 'n-checkpoint', 'n-end'];
    sequence.forEach((nodeId, idx) => {
      setTimeout(() => {
        addRunEvent({
          id: `evt-${Date.now()}-${idx}`,
          job_id: 'ui-demo-run',
          node_id: nodeId,
          event_type: 'command_sent',
          attempt_no: 1,
          at: new Date().toISOString(),
          payload_json: JSON.stringify({ node: nodeId, status: 'dispatched' }),
        });

        setTimeout(() => {
          addRunEvent({
            id: `evt-ack-${Date.now()}-${idx}`,
            job_id: 'ui-demo-run',
            node_id: nodeId,
            event_type: 'device_ack',
            attempt_no: 1,
            at: new Date().toISOString(),
            payload_json: JSON.stringify({ node: nodeId, status: 'ack_verified' }),
          });
          if (idx === sequence.length - 1) {
            setIsRunning(false);
          }
        }, 300);
      }, idx * 700);
    });
  };

  const handleStop = () => {
    setIsRunning(false);
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-[#101722] text-[#EDF3FC]">
      <AppTitlebar />
      <AppCommandBar
        currentView={currentView}
        onValidate={handleValidate}
        onRun={handleRun}
        onStop={handleStop}
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
