import { useState } from 'react';
import { parseSmartPaste } from '../../api/skillHub';
import { errorMessage } from '../../utils/errorMessage';
import type { DiscoverableSkill, SmartPastePreview } from '../../model/types';
import InstallConfirmDialog from './InstallConfirmDialog';

const SMART_PASTE_GITHUB_EXAMPLE =
  'https://github.com/obra/superpowers/blob/main/skills/brainstorming/SKILL.md';

export interface SmartPasteBarProps {
  disabled?: boolean;
  onInstall: (skill: DiscoverableSkill) => Promise<void>;
  onError?: (error: unknown) => void;
}

function previewToDiscoverable(preview: SmartPastePreview): DiscoverableSkill {
  return {
    key: `${preview.repoHost}/${preview.projectPath}:${preview.directory}`,
    name: preview.name,
    description: preview.description,
    directory: preview.directory,
    installDirName: preview.installDirName,
    repoHost: preview.repoHost,
    projectPath: preview.projectPath,
    repoOwner: preview.repoOwner,
    repoName: preview.repoName,
    repoBranch: preview.repoBranch,
    source: preview.source,
  };
}

function SmartPasteBar(props: SmartPasteBarProps) {
  const { disabled = false, onInstall, onError } = props;
  const [input, setInput] = useState('');
  const [parsing, setParsing] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [preview, setPreview] = useState<SmartPastePreview | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const handleParse = async () => {
    const value = input.trim();
    if (!value) {
      onError?.('请输入链接');
      return;
    }

    setParsing(true);
    try {
      const result = await parseSmartPaste(value);
      setPreview(result);
      setDialogOpen(true);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setParsing(false);
    }
  };

  const handleConfirm = async () => {
    if (!preview) return;
    setInstalling(true);
    try {
      await onInstall(previewToDiscoverable(preview));
      setInput('');
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setDialogOpen(false);
      setPreview(null);
      setInstalling(false);
    }
  };

  const handleCancel = () => {
    setDialogOpen(false);
    setPreview(null);
  };

  return (
    <>
      <div className="smart-paste-card">
        <label htmlFor="smartPasteInput">粘贴链接快速安装</label>
        <div className="smart-paste-row">
          <input
            type="text"
            id="smartPasteInput"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void handleParse();
            }}
            placeholder="粘贴 GitHub / skills.sh 链接"
            aria-label="Smart Paste 链接"
            aria-describedby="smartPasteHint"
            disabled={disabled || parsing || installing}
          />
          <button
            type="button"
            className="btn-icon"
            onClick={() => void handleParse()}
            aria-label="安装"
            disabled={disabled || parsing || installing}
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              aria-hidden="true"
            >
              <path d="M12 19V5" />
              <path d="m5 12 7-7 7 7" />
            </svg>
          </button>
        </div>
        <p id="smartPasteHint" className="smart-paste-hint">
          示例：{SMART_PASTE_GITHUB_EXAMPLE}
        </p>
      </div>
      <InstallConfirmDialog
        open={dialogOpen}
        preview={preview}
        installing={installing}
        onConfirm={() => void handleConfirm()}
        onCancel={handleCancel}
      />
    </>
  );
}

export default SmartPasteBar;
