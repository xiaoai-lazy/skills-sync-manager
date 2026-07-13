import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import GitLabPatDialog from '../components/skill-hub/GitLabPatDialog';

describe('GitLabPatDialog', () => {
  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <GitLabPatDialog
        open={false}
        host="gitlab.example.com"
        description="gitlab.example.com/acme/tools"
        onClose={vi.fn()}
        onSubmit={vi.fn()}
        submitLabel="验证并添加"
      />,
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('disables submit button when PAT input is empty', () => {
    render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com/acme/tools"
        onClose={vi.fn()}
        onSubmit={vi.fn()}
        submitLabel="验证并添加"
      />,
    );

    expect(screen.getByRole('button', { name: '验证并添加' })).toBeDisabled();
  });

  it('enables submit button when PAT input has value', async () => {
    render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com/acme/tools"
        onClose={vi.fn()}
        onSubmit={vi.fn()}
        submitLabel="验证并添加"
      />,
    );

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('访问密钥（PAT）'), 'glpat-test');

    expect(screen.getByRole('button', { name: '验证并添加' })).toBeEnabled();
  });

  it('shows 验证中… while submitting', async () => {
    let resolveSubmit: () => void;
    const onSubmit = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveSubmit = resolve;
        }),
    );

    render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com/acme/tools"
        onClose={vi.fn()}
        onSubmit={onSubmit}
        submitLabel="验证并添加"
      />,
    );

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('访问密钥（PAT）'), 'glpat-test');
    await user.click(screen.getByRole('button', { name: '验证并添加' }));

    expect(screen.getByRole('button', { name: '验证中…' })).toBeDisabled();

    resolveSubmit!();
    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith('glpat-test');
    });
  });

  it('cannot close from Escape or overlay while submitting', async () => {
    let resolveSubmit: () => void;
    const onSubmit = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveSubmit = resolve;
        }),
    );
    const onClose = vi.fn();
    const user = userEvent.setup();
    const { container } = render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com"
        mode="authenticate"
        onClose={onClose}
        onSubmit={onSubmit}
        submitLabel="验证并保存"
      />,
    );

    await user.type(screen.getByLabelText('访问密钥（PAT）'), 'glpat-test');
    await user.click(screen.getByRole('button', { name: '验证并保存' }));
    await user.keyboard('{Escape}');
    await user.click(container.querySelector('.credential-pat-overlay')!);

    expect(onClose).not.toHaveBeenCalled();
    resolveSubmit!();
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  });

  it('shows error after failed validation', async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error('访问密钥无效或权限不足，请检查后重试'));

    render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com/acme/tools"
        onClose={vi.fn()}
        onSubmit={onSubmit}
        submitLabel="验证并添加"
      />,
    );

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('访问密钥（PAT）'), 'bad-token');
    await user.click(screen.getByRole('button', { name: '验证并添加' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('访问密钥无效或权限不足，请检查后重试');
    });
  });

  it('calls onClose when cancel is clicked', async () => {
    const onClose = vi.fn();

    render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com/acme/tools"
        onClose={onClose}
        onSubmit={vi.fn()}
        submitLabel="验证并添加"
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '取消' }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('describes host authentication without claiming the repository is private', () => {
    render(
      <GitLabPatDialog
        open={true}
        host="gitlab.example.com"
        description="gitlab.example.com"
        mode="authenticate"
        onClose={vi.fn()}
        onSubmit={vi.fn()}
        submitLabel="验证并保存"
      />,
    );

    expect(screen.getByText(/为 GitLab 站点/)).toHaveTextContent('gitlab.example.com');
    expect(screen.queryByText(/需要登录后访问/)).not.toBeInTheDocument();
    expect(screen.getByRole('dialog')).toHaveClass('credential-pat-overlay');
  });
});
