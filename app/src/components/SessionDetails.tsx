import {
  ArrowClockwise,
  CaretDown,
  CaretUp,
  ChatCenteredDots,
  FolderOpen,
  WarningCircle,
} from "@phosphor-icons/react";
import { useState } from "react";
import { backend } from "../lib/backend";
import {
  groupSessionsByProject,
  type SessionProjectGroup,
} from "../lib/sessionGroups";
import type { SessionSort } from "../lib/sessionSort";
import type {
  LocalSessionView,
  SessionSummary,
  TokenBreakdown,
} from "../types/dashboard";

interface SessionDetailsProps {
  view: LocalSessionView;
}

const compactNumber = new Intl.NumberFormat("zh-CN", {
  notation: "compact",
  maximumFractionDigits: 1,
});

const fullNumber = new Intl.NumberFormat("zh-CN");

function totalTokens(tokens: TokenBreakdown) {
  return tokens.inputTokens + tokens.outputTokens;
}

function formatActivity(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1_000));
}

function formatCost(cost: number | null, unpricedTokens: number) {
  if (cost === null) return "费用待定";
  const prefix = unpricedTokens > 0 ? "≥" : "≈";
  const digits = cost < 1 ? 2 : 1;
  return `${prefix} US$${cost.toFixed(digits)}`;
}

function modelLabel(group: SessionProjectGroup) {
  if (group.models.length === 0) return "未知模型";
  if (group.models.length === 1) return group.models[0];
  return `${group.models.length} 个模型`;
}

export function SessionDetails({ view }: SessionDetailsProps) {
  const [expandedProject, setExpandedProject] = useState<string | null>(null);
  const [rescanning, setRescanning] = useState(false);
  const [sort, setSort] = useState<SessionSort>("recent");
  const projects = groupSessionsByProject(view.sessions, sort);

  const rescan = async () => {
    setRescanning(true);
    try {
      await backend.rescanSessions();
    } finally {
      setRescanning(false);
    }
  };

  return (
    <section
      className="panel sessions-panel"
      aria-labelledby="sessions-heading"
      data-section="sessions"
    >
      <div className="panel-heading">
        <div>
          <span className="eyebrow">本机记录</span>
          <h2 id="sessions-heading">项目与会话</h2>
        </div>
        {view.sessions.length > 0 && (
          <span className="session-scan-time">
            {projects.length} 个项目 · {view.sessions.length} 个会话
          </span>
        )}
      </div>

      {view.diagnostics.lastError ? (
        <div className="session-error">
          <span className="session-empty-icon">
            <WarningCircle size={22} />
          </span>
          <div>
            <strong>无法读取本机会话记录</strong>
            <p>{view.diagnostics.lastError}</p>
          </div>
          <button type="button" onClick={() => void rescan()} disabled={rescanning}>
            <ArrowClockwise
              className={rescanning ? "refreshing" : undefined}
              size={16}
            />
            重新扫描
          </button>
        </div>
      ) : view.sessions.length === 0 ? (
        <div className="session-empty">
          <span className="session-empty-icon">
            <ChatCenteredDots size={22} />
          </span>
          <div>
            <strong>本机尚未发现会话记录</strong>
            <p>账号额度仍覆盖所有设备，会话明细仅统计当前电脑。</p>
          </div>
        </div>
      ) : (
        <div
          className="session-table-wrap"
          style={{ overflowX: "hidden", overflowY: "auto" }}
        >
          <table
            className="session-table project-table"
            style={{ tableLayout: "fixed" }}
          >
            <thead>
              <tr>
                <th>项目</th>
                <th>会话</th>
                <th>模型</th>
                <th>
                  <button
                    className={sort.startsWith("tokens") ? "active" : ""}
                    type="button"
                    aria-label={
                      sort === "tokensDesc"
                        ? "Token，降序"
                        : sort === "tokensAsc"
                          ? "Token，升序"
                          : "Token"
                    }
                    onClick={() =>
                      setSort(current =>
                        current === "tokensDesc" ? "tokensAsc" : "tokensDesc",
                      )
                    }
                  >
                    本月 Token
                    {sort === "tokensDesc" ? (
                      <CaretDown size={10} />
                    ) : sort === "tokensAsc" ? (
                      <CaretUp size={10} />
                    ) : null}
                  </button>
                </th>
                <th>本月等效费用</th>
                <th>
                  <button
                    className={sort === "recent" ? "active" : ""}
                    type="button"
                    aria-label="最后活动"
                    onClick={() => setSort("recent")}
                  >
                    最后活动
                    {sort === "recent" && <CaretDown size={10} />}
                  </button>
                </th>
              </tr>
            </thead>
            <tbody>
              {projects.map(project => (
                <ProjectRow
                  key={project.id}
                  project={project}
                  expanded={expandedProject === project.id}
                  onToggle={() =>
                    setExpandedProject(current =>
                      current === project.id ? null : project.id,
                    )
                  }
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function ProjectRow({
  project,
  expanded,
  onToggle,
}: {
  project: SessionProjectGroup;
  expanded: boolean;
  onToggle: () => void;
}) {
  const detailsId = `project-details-${project.id.replace(/[^a-z0-9_-]/gi, "-")}`;
  return (
    <>
      <tr className="session-row project-row">
        <td>
          <button
            className="session-title-button project-title-button"
            type="button"
            aria-expanded={expanded}
            aria-controls={detailsId}
            onClick={onToggle}
          >
            <CaretDown className={expanded ? "expanded" : undefined} size={14} />
            <span className="project-folder-icon">
              <FolderOpen size={16} />
            </span>
            <span>
              <strong title={project.projectPath ?? project.name}>{project.name}</strong>
              <small title={project.projectPath ?? "未知路径"}>
                {project.projectPath ?? "未知路径"}
              </small>
            </span>
          </button>
        </td>
        <td>{project.sessions.length}</td>
        <td className="session-secondary" title={project.models.join(", ")}>
          {modelLabel(project)}
        </td>
        <td>{compactNumber.format(totalTokens(project.monthlyTokens))}</td>
        <td>
          {formatCost(
            project.monthlyEquivalentCostUsd,
            project.monthlyUnpricedTokens,
          )}
        </td>
        <td className="session-secondary">{formatActivity(project.lastActiveAt)}</td>
      </tr>
      {expanded && (
        <tr className="session-detail-row project-detail-row" id={detailsId}>
          <td colSpan={6}>
            <div className="project-session-header">
              <span>{project.sessions.length} 个会话，按最近活动排序</span>
              {project.childSessionCount > 0 && (
                <span>包含 {project.childSessionCount} 个子任务</span>
              )}
            </div>
            <div className="project-session-list">
              {project.sessions.map(session => (
                <SessionDisclosure key={session.sessionId} session={session} />
              ))}
            </div>
          </td>
        </tr>
      )}
    </>
  );
}

function SessionDisclosure({ session }: { session: SessionSummary }) {
  return (
    <details className="project-session-item">
      <summary>
        <span className="session-time">
          <strong>{formatActivity(session.lastActiveAt)}</strong>
          <small>{session.sessionId.slice(-8)}</small>
        </span>
        <span>{session.primaryModel ?? "未知模型"}</span>
        <span>
          {compactNumber.format(
            totalTokens(session.monthlyTokens ?? session.tokens),
          )}
        </span>
        <span>
          {formatCost(
            session.monthlyEquivalentCostUsd === undefined
              ? session.equivalentCostUsd
              : session.monthlyEquivalentCostUsd,
            session.monthlyUnpricedTokens ?? session.unpricedTokens,
          )}
        </span>
        <CaretDown className="session-disclosure-caret" size={13} />
      </summary>
      <dl className="session-breakdown project-session-breakdown">
        <div>
          <dt>历史总 Token</dt>
          <dd>{fullNumber.format(totalTokens(session.tokens))}</dd>
        </div>
        <div>
          <dt>历史总等效费用</dt>
          <dd>{formatCost(session.equivalentCostUsd, session.unpricedTokens)}</dd>
        </div>
        <div>
          <dt>历史输入</dt>
          <dd>{fullNumber.format(session.tokens.inputTokens)}</dd>
        </div>
        <div>
          <dt>历史缓存输入</dt>
          <dd>{fullNumber.format(session.tokens.cachedInputTokens)}</dd>
        </div>
        <div>
          <dt>历史输出</dt>
          <dd>{fullNumber.format(session.tokens.outputTokens)}</dd>
        </div>
        <div>
          <dt>历史推理输出</dt>
          <dd>{fullNumber.format(session.tokens.reasoningOutputTokens)}</dd>
        </div>
        {session.unpricedTokens > 0 && (
          <div>
            <dt>未定价</dt>
            <dd>{fullNumber.format(session.unpricedTokens)} 未定价 Token</dd>
          </div>
        )}
      </dl>
    </details>
  );
}
