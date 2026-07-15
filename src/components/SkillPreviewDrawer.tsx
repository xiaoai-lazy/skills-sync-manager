import { useCallback, useEffect, useRef, useState } from 'react';
import { readSkillMarkdown } from '../api/skillHub';
import { useModalFocus } from '../hooks/useModalFocus';
import type { SkillMarkdownPreview, SkillMarkdownRequest } from '../model/types';
import { errorMessage } from '../utils/errorMessage';
import { SkillMarkdownView } from './SkillMarkdownView';

export interface SkillPreviewDrawerProps {
  open: boolean;
  request: SkillMarkdownRequest | null;
  onClose: () => void;
}

function SkillPreviewDrawer(props: SkillPreviewDrawerProps) {
  const { open, request, onClose } = props;
  const drawerRef = useRef<HTMLDivElement>(null);
  const generationRef = useRef(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<SkillMarkdownPreview | null>(null);
  const [retryToken, setRetryToken] = useState(0);
  const requestIdentity =
    request == null
      ? null
      : request.kind === 'installed'
        ? `installed:${request.storageKey}`
        : `discover:${request.discoverKey}`;

  useModalFocus({
    open,
    containerRef: drawerRef,
    onEscape: onClose,
  });

  const loadPreview = useCallback(async (activeRequest: SkillMarkdownRequest) => {
    const generation = ++generationRef.current;
    setLoading(true);
    setError(null);
    setPreview(null);
    try {
      const result = await readSkillMarkdown(activeRequest);
      if (generation !== generationRef.current) return;
      setPreview(result);
    } catch (err) {
      if (generation !== generationRef.current) return;
      setError(errorMessage(err));
    } finally {
      if (generation === generationRef.current) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (!open || !request) {
      generationRef.current += 1;
      setLoading(false);
      setError(null);
      setPreview(null);
      return;
    }
    void loadPreview(request);
  }, [open, request, requestIdentity, retryToken, loadPreview]);

  const handleRetry = () => {
    setRetryToken((token) => token + 1);
  };

  if (!open) return null;

  return (
    <div
      className="overlay drawer-overlay open"
      role="dialog"
      aria-modal="true"
      aria-label="Skill 预览"
      onClick={onClose}
    >
      <div
        ref={drawerRef}
        className="drawer skill-preview-drawer"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="drawer-header-row">
          <div>
            <p className="skill-preview-eyebrow">Skill 预览</p>
            <h2 data-testid="skill-preview-title">{preview?.title ?? (loading ? '加载中…' : '—')}</h2>
            <p className="drawer-subtitle" data-testid="skill-preview-description">
              {preview?.description ?? ''}
            </p>
          </div>
          <button
            type="button"
            className="skill-preview-close"
            aria-label="关闭预览"
            onClick={onClose}
          >
            ×
          </button>
        </div>

        <div className="skill-preview-body">
          {loading ? (
            <div
              className="skill-preview-skeleton"
              data-testid="skill-preview-skeleton"
              aria-busy="true"
              aria-label="加载中"
            >
              <div className="skill-preview-skeleton-line" />
              <div className="skill-preview-skeleton-line" />
              <div className="skill-preview-skeleton-line short" />
            </div>
          ) : error ? (
            <div className="skill-preview-error" role="alert">
              <p>{error}</p>
              <button type="button" className="secondary-button" onClick={handleRetry}>
                重试
              </button>
            </div>
          ) : preview ? (
            <SkillMarkdownView markdown={preview.markdownBody} />
          ) : null}
        </div>

        <div className="drawer-footer">
          <button type="button" onClick={onClose}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}

export default SkillPreviewDrawer;
