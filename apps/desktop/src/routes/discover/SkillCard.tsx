import type { SkillItem } from "@skills-pp/shared";
import { ExternalLink, Star } from "lucide-react";
import { useNavigate } from "react-router-dom";

interface Props {
  skill: SkillItem;
}

export function SkillCard({ skill }: Props) {
  const navigate = useNavigate();

  return (
    <button
      className="w-full rounded-lg border border-gray-200 bg-white p-4 text-left transition-shadow hover:shadow-md"
      onClick={() => navigate(`/skill/${encodeURIComponent(skill.id)}`)}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-semibold text-gray-900">
              {skill.name}
            </span>
            {skill.stars != null && (
              <span className="flex items-center gap-1 text-xs text-yellow-600">
                <Star className="h-3 w-3" />
                {skill.stars}
              </span>
            )}
          </div>
          {skill.author && (
            <p className="mt-0.5 text-xs text-gray-400">by {skill.author}</p>
          )}
          {skill.description && (
            <p className="mt-1 line-clamp-2 text-xs text-gray-500">
              {skill.description}
            </p>
          )}
          <div className="mt-2 flex flex-wrap gap-1">
            {skill.tags.slice(0, 3).map((tag) => (
              <span
                key={tag}
                className="rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-500"
              >
                {tag}
              </span>
            ))}
            {skill.compatibleTools?.slice(0, 2).map((tool) => (
              <span
                key={tool}
                className="rounded-full bg-brand-50 px-2 py-0.5 text-xs text-brand-700"
              >
                {tool}
              </span>
            ))}
          </div>
        </div>
        <ExternalLink className="mt-0.5 h-3.5 w-3.5 shrink-0 text-gray-300" />
      </div>
    </button>
  );
}
