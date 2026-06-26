import React from 'react';
import type { Target } from '../model/types';
import MainLibraryPanel from './MainLibraryPanel';
import TargetList from './TargetList';

export interface SidebarProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  targets: Target[];
  selectedTargetId: string | null;
  onSelectTarget: (targetId: string) => void;
  onAddTarget: () => void;
  onEditTarget: (target: Target) => void;
  onDeleteTarget: (target: Target) => void;
  onSetMainSkillsDir: () => void;
}

function Sidebar(props: SidebarProps) {
  return (
    <aside className="sidebar">
      <MainLibraryPanel
        mainSkillsDir={props.mainSkillsDir}
        validSkillCount={props.validSkillCount}
        invalidSkillCount={props.invalidSkillCount}
        onSetMainSkillsDir={props.onSetMainSkillsDir}
      />
      <TargetList
        targets={props.targets}
        selectedTargetId={props.selectedTargetId}
        onSelectTarget={props.onSelectTarget}
        onAddTarget={props.onAddTarget}
        onEditTarget={props.onEditTarget}
        onDeleteTarget={props.onDeleteTarget}
      />
    </aside>
  );
}

export default Sidebar;
