import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

export interface SkillMarkdownViewProps {
  markdown: string;
}

export function SkillMarkdownView({ markdown }: SkillMarkdownViewProps) {
  return (
    <div className="skill-markdown-view">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>
    </div>
  );
}

export default SkillMarkdownView;
