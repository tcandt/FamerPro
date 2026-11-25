import React, { useState, useEffect } from 'react';
import { AppShell } from './components/shell/AppShell';
import { ProjectsDashboardView } from './features/projects/ProjectsDashboardView';
import { OverviewView } from './features/overview/OverviewView';
import { DevicesView } from './features/devices/DevicesView';
import { DeviceStudioView } from './features/devices/DeviceStudioView';
import { WorkflowsView } from './features/workflows/WorkflowsView';
import { WorkflowStudioView } from './features/studio/WorkflowStudioView';
import { TasksView } from './features/tasks/TasksView';
import { RunsView } from './features/runs/RunsView';
import { MiniToolBuilderView } from './features/builder/MiniToolBuilderView';
import { AccountsView } from './features/accounts/AccountsView';
import { NetworkView } from './features/network/NetworkView';
import { SettingsView } from './features/settings/SettingsView';
import { ComponentGalleryView } from './features/gallery/ComponentGalleryView';

export const App: React.FC = () => {
  const [currentView, setCurrentView] = useState(() => {
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      const viewParam = params.get('view');
      if (viewParam) return viewParam;
      if (window.location.hash) {
        const hash = window.location.hash.replace('#', '').split('/')[0];
        if (hash) return hash;
      }
    }
    return 'device-studio';
  });

  useEffect(() => {
    (window as any).__setFarmerProView = (view: string) => {
      setCurrentView(view);
    };

    const handleHash = () => {
      const hash = window.location.hash.replace('#', '').split('/')[0];
      if (hash) setCurrentView(hash);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey) {
        if (e.key === 'D') setCurrentView('devices');
        if (e.key === 'S') setCurrentView('device-studio');
        if (e.key === 'P') setCurrentView('projects');
      }
    };

    window.addEventListener('hashchange', handleHash);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('hashchange', handleHash);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  return (
    <AppShell currentView={currentView} onSelectView={setCurrentView}>
      {currentView === 'projects' && <ProjectsDashboardView onNavigate={setCurrentView} />}
      {currentView === 'overview' && <OverviewView onNavigate={setCurrentView} />}
      {currentView === 'devices' && <DevicesView onNavigate={setCurrentView} />}
      {currentView === 'device-studio' && <DeviceStudioView onNavigate={setCurrentView} />}
      {currentView === 'workflows' && (
        <WorkflowsView onOpenStudio={() => setCurrentView('studio')} />
      )}
      {currentView === 'studio' && <WorkflowStudioView />}
      {currentView === 'tasks' && <TasksView />}
      {currentView === 'runs' && <RunsView />}
      {currentView === 'builder' && <MiniToolBuilderView onNavigate={setCurrentView} />}
      {currentView === 'accounts' && <AccountsView />}
      {currentView === 'network' && <NetworkView />}
      {currentView === 'settings' && <SettingsView />}
      {currentView === 'gallery' && <ComponentGalleryView />}
    </AppShell>
  );
};

export default App;
