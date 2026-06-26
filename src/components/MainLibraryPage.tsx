import React from 'react';
import type { SkillView } from '../model/types';

export interface MainLibraryPageProps {
  skills: SkillView[];
  validSkillCount: number;
  invalidSkillCount: number;
  onDeleteMainSkill: (skillDirName: string) => void;
}

function MainLibraryPage(props: MainLibraryPageProps) {
  return (
    <section className="main-library-page">
      <h2>Main Library</h2>
      <div className="skill-counts" style={{ marginBottom: '1rem' }}>
        <span className="count valid">{props.validSkillCount} valid</span>
        {props.invalidSkillCount > 0 && (
          <span className="count invalid">{props.invalidSkillCount} invalid</span>
        )}
      </div>

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
    </section>
  );
}

export default MainLibraryPage;
