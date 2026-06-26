import React from 'react';

export interface MainLibrarySummaryProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  onSetMainSkillsDir: () => void;
  onManageSkills: () => void;
}

function MainLibrarySummary(props: MainLibrarySummaryProps) {
  return (
    <section className="main-library-panel">
      <h2>Main Library</h2>
      {props.mainSkillsDir ? (
        <div className="dir-info">
          <div className="dir-path" title={props.mainSkillsDir}>
            {props.mainSkillsDir}
          </div>
          <div className="skill-counts">
            <span className="count valid">{props.validSkillCount} valid</span>
            {props.invalidSkillCount > 0 && (
              <span className="count invalid">
                {props.invalidSkillCount} invalid
              </span>
            )}
          </div>
        </div>
      ) : (
        <div className="empty-state">
          <button onClick={props.onSetMainSkillsDir}>Set Main Directory</button>
        </div>
      )}
      {props.mainSkillsDir && (
        <>
          <button className="secondary-button" onClick={props.onSetMainSkillsDir}>
            Change Directory
          </button>
          <button className="secondary-button" onClick={props.onManageSkills}>
            Manage Skills
          </button>
        </>
      )}
    </section>
  );
}

export default MainLibrarySummary;
