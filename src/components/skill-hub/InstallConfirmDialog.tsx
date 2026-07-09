import Dialog from '../Dialog';
import type { DiscoverableSkill, SmartPastePreview } from '../../model/types';
import { formatSkillSourceLabel } from '../../utils/skillSourceLabel';

export type InstallPreview = DiscoverableSkill | SmartPastePreview;

export interface InstallConfirmDialogProps {
  open: boolean;
  preview: InstallPreview | null;
  batchCount?: number;
  installing?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

function getPreviewName(preview: InstallPreview): string {
  return preview.name;
}

function InstallConfirmDialog(props: InstallConfirmDialogProps) {
  const { open, preview, batchCount = 1, installing = false, onConfirm, onCancel } = props;

  const isBatch = batchCount > 1;
  const title = isBatch ? `批量安装 (${batchCount})` : '安装 Skill';

  let message = '将保存到主库目录。';
  if (isBatch) {
    message = `安装 ${batchCount} 个 Skill 到主库。`;
  } else if (preview) {
    message = `安装 ${getPreviewName(preview)} 到主库。`;
  }

  return (
    <Dialog
      open={open}
      title={title}
      onClose={onCancel}
      actions={
        <>
          <button type="button" className="secondary-button" onClick={onCancel} disabled={installing}>
            取消
          </button>
          <button type="button" className="btn-primary" onClick={onConfirm} disabled={installing}>
            {installing ? '安装中…' : '安装'}
          </button>
        </>
      }
    >
      <p style={{ margin: 0 }}>{message}</p>
      {!isBatch && preview && (
        <dl className="install-preview-details">
          <div>
            <dt>目录</dt>
            <dd>{preview.installDirName}</dd>
          </div>
          <div>
            <dt>来源</dt>
            <dd>{formatSkillSourceLabel(preview.source)}</dd>
          </div>
          {'projectPath' in preview && preview.projectPath && (
            <div>
              <dt>仓库</dt>
              <dd>
                {preview.source === 'gitlab'
                  ? `${preview.repoHost}/${preview.projectPath}`
                  : preview.projectPath}
              </dd>
            </div>
          )}
        </dl>
      )}
    </Dialog>
  );
}

export default InstallConfirmDialog;
