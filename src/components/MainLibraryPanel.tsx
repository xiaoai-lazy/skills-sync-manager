import React from 'react';
import type { SkillView } from '../model/types';

export interface MainLibraryPanelProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  skills: SkillView[];
  onSetMainSkillsDir: () => void;
  onDeleteMainSkill: (skillDirName: string) => void;
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

      {props.mainSkillsDir && (
        <div className="library-skill-list">
          <h3>All Skills ({props.skills.length})</h3>
          {props.skills.length === 0 ? (
            <div className="empty-state">
              <p>No skills found in the main directory.</p>
            </div>
          ) : (
            <ul className="skill-list">
              {props.skills.map((skill) => (
                <li key={skill.dirName}>
                  <div className={`skill-row ${!skill.valid ? 'invalid' : ''}`}>
                    <div className="skill-info">
                      <div className="skill-name">{skill.name ?? skill.dirName}</div>
                      {skill.description && (
                        <div className="skill-description">{skill.description}</div>
                      )}
                      <div className="skill-dir">{skill.dirName}</div>
                      {!skill.valid && skill.validationErrors.length > 0 && (
                        <div className="skill-message">
                          {skill.validationErrors.join(', ')}
                        </div>
                      )}
                    </div>
                    <div className="skill-actions">
                      <button
                        className="danger-button"
                        onClick={() => props.onDeleteMainSkill(skill.dirName)}
                        aria-label={`Delete skill ${skill.dirName}`}
                        title="从主库删除"
                      >
                        删除
                      </button>
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}

export default MainLibraryPanel;
