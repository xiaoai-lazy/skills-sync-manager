export interface SkillListEmptyStateProps {
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
}

function SkillListEmptyState(props: SkillListEmptyStateProps) {
  const { title, description, actionLabel, onAction } = props;

  return (
    <div className="skill-list-empty" role="status">
      <div className="skill-list-empty-icon" aria-hidden="true">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <path d="M14 2v6h6M9 13h6M9 17h4" />
        </svg>
      </div>
      <p className="skill-list-empty-title">{title}</p>
      <p className="skill-list-empty-desc">{description}</p>
      {actionLabel && onAction ? (
        <button type="button" className="btn-sm skill-list-empty-action" onClick={onAction}>
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}

export default SkillListEmptyState;
