import type { Target } from '../model/types';

export interface TargetRowProps {
  target: Target;
  isSelected: boolean;
  onSelect: () => void;
  onEdit: (target: Target) => void;
  onDelete: (target: Target) => void;
}

function TargetRow(props: TargetRowProps) {
  const showEdit = props.target.kind !== 'agent';

  return (
    <li
      className={`target-item ${props.isSelected ? 'selected' : ''}`}
      onClick={props.onSelect}
    >
      <span className="target-dot" />
      <span className="target-name" title={props.target.skillsDir}>
        {props.target.name}
      </span>
      <div className="target-actions">
        {showEdit && (
          <button
            type="button"
            className="icon-button"
            onClick={(e) => {
              e.stopPropagation();
              props.onEdit(props.target);
            }}
            aria-label={`Edit target ${props.target.name}`}
          >
            ✎
          </button>
        )}
        <button
          type="button"
          className="icon-button danger-button"
          onClick={(e) => {
            e.stopPropagation();
            props.onDelete(props.target);
          }}
          aria-label={`Delete target ${props.target.name}`}
        >
          🗑
        </button>
      </div>
    </li>
  );
}

export default TargetRow;
