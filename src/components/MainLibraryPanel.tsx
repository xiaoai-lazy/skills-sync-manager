import React from 'react';

export interface MainLibraryPanelProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  onSetMainSkillsDir: () => void;
}

function MainLibraryPanel(props: MainLibraryPanelProps) {
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
          <p>No main skills directory configured.</p>
          <button onClick={props.onSetMainSkillsDir}>Set Main Directory</button>
        </div>
      )}
      {props.mainSkillsDir && (
        <button className="secondary-button" onClick={props.onSetMainSkillsDir}>
          Change Directory
        </button>
      )}
    </section>
  );
}

export default MainLibraryPanel;
