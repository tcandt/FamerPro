import React, { useState } from 'react';
import { AppShell } from './components/shell/AppShell';
import { OverviewView } from './features/overview/OverviewView';
import { DevicesView } from './features/devices/DevicesView';
import { WorkflowsView } from './features/workflows/WorkflowsView';
import { WorkflowStudioView } from './features/studio/WorkflowStudioView';
import { TasksView } from './features/tasks/TasksView';
import { RunsView } from './features/runs/RunsView';
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
        const hash = window.location.hash.replace('#', '');
        if (hash) return hash;
      }
    }
    return 'overview';
  });

  return (
    <AppShell currentView={currentView} onSelectView={setCurrentView}>
      {currentView === 'overview' && <OverviewView onNavigate={setCurrentView} />}
      {currentView === 'devices' && <DevicesView onNavigate={setCurrentView} />}
      {currentView === 'workflows' && <WorkflowsView onOpenStudio={() => setCurrentView('studio')} />}
      {currentView === 'studio' && <WorkflowStudioView />}
      {currentView === 'tasks' && <TasksView />}
      {currentView === 'runs' && <RunsView />}
      {currentView === 'accounts' && <AccountsView />}
      {currentView === 'network' && <NetworkView />}
      {currentView === 'settings' && <SettingsView />}
      {currentView === 'gallery' && <ComponentGalleryView />}
    </AppShell>
  );
};

export default App;
