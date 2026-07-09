export interface SkillListEmptyStateProps {
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
  /** Defaults to true when description is set, false for title-only empty states. */
  showIcon?: boolean;
}

function SkillListEmptyState(props: SkillListEmptyStateProps) {
  const { title, description, actionLabel, onAction } = props;
  const showIcon = props.showIcon ?? Boolean(description);

  return (
    <div className="skill-list-empty" role="status">
      {showIcon ? (
        <div className="skill-list-empty-icon" aria-hidden="true">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <path d="M14 2v6h6M9 13h6M9 17h4" />
          </svg>
        </div>
      ) : null}
      <p className="skill-list-empty-title">{title}</p>
      {description ? <p className="skill-list-empty-desc">{description}</p> : null}
      {actionLabel && onAction ? (
        <button type="button" className="btn-sm skill-list-empty-action" onClick={onAction}>
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}

export default SkillListEmptyState;
