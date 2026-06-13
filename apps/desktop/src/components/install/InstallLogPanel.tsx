import { useState } from "react";
import { CheckCircle, XCircle, ChevronDown, ChevronUp } from "lucide-react";
import type { InstallTaskResult } from "@skills-pp/shared";

interface Props {
  task: InstallTaskResult;
}

export function InstallLogPanel({ task }: Props) {
  const [expanded, setExpanded] = useState(false);

  const isSuccess = task.status === "success";
  const haslogs = task.logLines && task.logLines.length > 0;

  return (
    <div
      className={`rounded-lg border p-4 ${
        isSuccess
          ? "border-green-200 bg-green-50"
          : "border-red-200 bg-red-50"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          {isSuccess ? (
            <CheckCircle className="h-4 w-4 shrink-0 text-green-600" />
          ) : (
            <XCircle className="h-4 w-4 shrink-0 text-red-600" />
          )}
          <div>
            <p className={`text-sm font-medium ${isSuccess ? "text-green-900" : "text-red-900"}`}>
              {isSuccess
                ? `「${task.skillName}」安装成功`
                : `「${task.skillName}」安装失败`}
            </p>
            {!isSuccess && task.errorMessage && (
              <p className="mt-0.5 text-xs text-red-700">{task.errorMessage}</p>
            )}
          </div>
        </div>

        {haslogs && (
          <button
            onClick={() => setExpanded((v) => !v)}
            className={`flex items-center gap-1 text-xs ${
              isSuccess ? "text-green-700 hover:text-green-900" : "text-red-700 hover:text-red-900"
            }`}
          >
            {expanded ? (
              <>
                收起 <ChevronUp className="h-3 w-3" />
              </>
            ) : (
              <>
                详细日志 <ChevronDown className="h-3 w-3" />
              </>
            )}
          </button>
        )}
      </div>

      {expanded && haslogs && (
        <pre className="mt-3 max-h-48 overflow-auto rounded bg-black/10 p-3 text-xs leading-relaxed text-gray-700">
          {task.logLines.join("\n")}
        </pre>
      )}
    </div>
  );
}
