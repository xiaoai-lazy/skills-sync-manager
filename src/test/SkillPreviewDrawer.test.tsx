import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { SkillMarkdownPreview, SkillMarkdownRequest } from '../model/types';
import SkillPreviewDrawer from '../components/SkillPreviewDrawer';

const readSkillMarkdownMock = vi.fn();

vi.mock('../api/skillHub', () => ({
  readSkillMarkdown: (...args: unknown[]) => readSkillMarkdownMock(...args),
}));

const samplePreview: SkillMarkdownPreview = {
  title: 'Brainstorming',
  description: 'Explore ideas before implementation.',
  markdownBody: '# Brainstorming\n\nUse this skill to explore ideas.',
  origin: 'mainLibrary',
};

const installedRequest: SkillMarkdownRequest = {
  kind: 'installed',
  storageKey: 'brainstorming',
};

const discoverRequest: SkillMarkdownRequest = {
  kind: 'discover',
  discoverKey: 'anthropics/skills:skills/brainstorming',
};

function renderDrawer(
  overrides: Partial<React.ComponentProps<typeof SkillPreviewDrawer>> = {},
) {
  const onClose = vi.fn();
  const props = {
    open: true,
    request: installedRequest as SkillMarkdownRequest | null,
    onClose,
    ...overrides,
  };
  return { ...render(<SkillPreviewDrawer {...props} />), ...props };
}

beforeEach(() => {
  readSkillMarkdownMock.mockReset();
  readSkillMarkdownMock.mockResolvedValue(samplePreview);
});

afterEach(() => cleanup());

describe('SkillPreviewDrawer', () => {
  it('opens with title「Skill 预览」and loads preview for the request', async () => {
    renderDrawer();

    const dialog = screen.getByRole('dialog', { name: 'Skill 预览' });
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText('Skill 预览')).toBeInTheDocument();

    await waitFor(() => {
      expect(readSkillMarkdownMock).toHaveBeenCalledWith(installedRequest);
    });
    expect(await screen.findByTestId('skill-preview-title')).toHaveTextContent('Brainstorming');
    expect(screen.getByTestId('skill-preview-description')).toHaveTextContent(
      'Explore ideas before implementation.',
    );
  });

  it('shows skeleton while the preview promise is pending', async () => {
    let resolvePreview!: (value: SkillMarkdownPreview) => void;
    readSkillMarkdownMock.mockImplementation(
      () =>
        new Promise<SkillMarkdownPreview>((resolve) => {
          resolvePreview = resolve;
        }),
    );

    renderDrawer();

    expect(screen.getByTestId('skill-preview-skeleton')).toBeInTheDocument();
    expect(screen.queryByText('Use this skill to explore ideas.')).not.toBeInTheDocument();

    resolvePreview(samplePreview);
    expect(await screen.findByText('Use this skill to explore ideas.')).toBeInTheDocument();
    expect(screen.queryByTestId('skill-preview-skeleton')).not.toBeInTheDocument();
  });

  it('renders markdown heading text from the body', async () => {
    renderDrawer();

    const heading = await screen.findByRole('heading', { level: 1, name: 'Brainstorming' });
    expect(heading).toBeInTheDocument();
  });

  it('shows「重试」on error and retries the API call', async () => {
    readSkillMarkdownMock
      .mockRejectedValueOnce(new Error('读取失败'))
      .mockResolvedValueOnce(samplePreview);

    const user = userEvent.setup();
    renderDrawer();

    expect(await screen.findByRole('alert')).toHaveTextContent('读取失败');
    const retry = screen.getByRole('button', { name: '重试' });
    await user.click(retry);

    await waitFor(() => {
      expect(readSkillMarkdownMock).toHaveBeenCalledTimes(2);
    });
    expect(await screen.findByText('Use this skill to explore ideas.')).toBeInTheDocument();
  });

  it('calls onClose from Escape, overlay, header close, and footer「关闭」', async () => {
    const user = userEvent.setup();
    const { container, onClose } = renderDrawer();

    await screen.findByText('Use this skill to explore ideas.');

    await user.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledTimes(1);

    onClose.mockClear();
    await user.click(screen.getByRole('button', { name: '关闭预览' }));
    expect(onClose).toHaveBeenCalledTimes(1);

    onClose.mockClear();
    await user.click(screen.getByRole('button', { name: '关闭' }));
    expect(onClose).toHaveBeenCalledTimes(1);

    onClose.mockClear();
    const overlay = container.querySelector('.drawer-overlay');
    expect(overlay).toBeInTheDocument();
    await user.click(overlay!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('reloads when request changes and ignores stale responses', async () => {
    let resolveFirst!: (value: SkillMarkdownPreview) => void;
    let resolveSecond!: (value: SkillMarkdownPreview) => void;

    readSkillMarkdownMock
      .mockImplementationOnce(
        () =>
          new Promise<SkillMarkdownPreview>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise<SkillMarkdownPreview>((resolve) => {
            resolveSecond = resolve;
          }),
      );

    const onClose = vi.fn();
    const { rerender } = render(
      <SkillPreviewDrawer open request={installedRequest} onClose={onClose} />,
    );

    await waitFor(() => expect(readSkillMarkdownMock).toHaveBeenCalledTimes(1));

    rerender(<SkillPreviewDrawer open request={discoverRequest} onClose={onClose} />);

    await waitFor(() => expect(readSkillMarkdownMock).toHaveBeenCalledTimes(2));
    expect(readSkillMarkdownMock).toHaveBeenLastCalledWith(discoverRequest);

    resolveFirst({
      ...samplePreview,
      title: 'Stale Title',
      markdownBody: '# Stale\n\nShould not appear.',
    });
    resolveSecond({
      ...samplePreview,
      title: 'Fresh Title',
      description: 'Fresh description',
      markdownBody: '# Fresh\n\nLatest content.',
    });

    expect(await screen.findByTestId('skill-preview-title')).toHaveTextContent('Fresh Title');
    expect(screen.getByText('Latest content.')).toBeInTheDocument();
    expect(screen.queryByText('Stale Title')).not.toBeInTheDocument();
    expect(screen.queryByText('Should not appear.')).not.toBeInTheDocument();
  });

  it('does not render when closed', () => {
    renderDrawer({ open: false });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(readSkillMarkdownMock).not.toHaveBeenCalled();
  });
});
