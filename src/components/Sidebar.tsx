import React from 'react';
import type { Target } from '../model/types';
import MainLibrarySummary from './MainLibrarySummary';
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
  onManageSkills: () => void;
}

function Sidebar(props: SidebarProps) {
  return (
    <aside className="sidebar">
      <MainLibrarySummary
        mainSkillsDir={props.mainSkillsDir}
        validSkillCount={props.validSkillCount}
        invalidSkillCount={props.invalidSkillCount}
        onSetMainSkillsDir={props.onSetMainSkillsDir}
        onManageSkills={props.onManageSkills}
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
