import React from 'react';
import type { Target } from '../model/types';

export interface TargetListProps {
  targets: Target[];
  selectedTargetId: string | null;
  onSelectTarget: (targetId: string) => void;
  onAddTarget: () => void;
  onEditTarget: (target: Target) => void;
  onDeleteTarget: (target: Target) => void;
}

function TargetList(props: TargetListProps) {
  return (
    <section className="target-list-section">
      <div className="target-list-header">
        <h2>Targets</h2>
        <button className="icon-button" onClick={props.onAddTarget} aria-label="Add target">
          +
        </button>
      </div>
      {props.targets.length === 0 ? (
        <div className="empty-state">
          <p>No targets configured.</p>
          <button onClick={props.onAddTarget}>Add Target</button>
        </div>
      ) : (
        <ul className="target-list">
          {props.targets.map((target) => (
            <li
              key={target.id}
              className={`target-item ${target.id === props.selectedTargetId ? 'selected' : ''}`}
              onClick={() => props.onSelectTarget(target.id)}
            >
              <div className="target-name" title={target.skillsDir}>
                {target.name}
              </div>
              <div className="target-actions">
                <button
                  className="icon-button"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onEditTarget(target);
                  }}
                  aria-label={`Edit target ${target.name}`}
                >
                  ✎
                </button>
                <button
                  className="icon-button danger-button"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onDeleteTarget(target);
                  }}
                  aria-label={`Delete target ${target.name}`}
                >
                  🗑
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export default TargetList;
