import React, { useState } from 'react';
import { AppShell } from './components/shell/AppShell';
import { OverviewView } from './features/overview/OverviewView';
import { DevicesView } from './features/devices/DevicesView';
import { WorkflowStudioView } from './features/studio/WorkflowStudioView';
import { TasksView } from './features/tasks/TasksView';
import { RunsView } from './features/runs/RunsView';
import { AccountsView } from './features/accounts/AccountsView';
import { NetworkView } from './features/network/NetworkView';
import { SettingsView } from './features/settings/SettingsView';
import { ComponentGalleryView } from './features/gallery/ComponentGalleryView';

export const App: React.FC = () => {
  const [currentView, setCurrentView] = useState('studio');

  return (
    <AppShell currentView={currentView} onSelectView={setCurrentView}>
      {currentView === 'overview' && <OverviewView onNavigate={setCurrentView} />}
      {currentView === 'devices' && <DevicesView />}
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
