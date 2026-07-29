import {
  ArrowClockwise,
  CaretDown,
  CaretUp,
  ChatCenteredDots,
  WarningCircle,
} from "@phosphor-icons/react";
import { useState } from "react";
import { backend } from "../lib/backend";
import {
  sortSessions,
  type SessionSort,
} from "../lib/sessionSort";
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

function projectName(path: string | null) {
  return path?.split(/[\\/]/).filter(Boolean).at(-1) ?? "未命名项目";
}

export function SessionDetails({ view }: SessionDetailsProps) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const [rescanning, setRescanning] = useState(false);
  const [sort, setSort] = useState<SessionSort>("recent");
  const sessions = sortSessions(view.sessions, sort);

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
          <h2 id="sessions-heading">会话详情</h2>
        </div>
        {view.sessions.length > 0 && (
          <span className="session-scan-time">
            {view.sessions.length} 个会话
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
          <table className="session-table" style={{ tableLayout: "fixed" }}>
            <thead>
              <tr>
                <th>会话</th>
                <th>项目</th>
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
              {sessions.map(session => (
                <SessionRow
                  key={session.sessionId}
                  session={session}
                  expanded={expanded === session.sessionId}
                  onToggle={() =>
                    setExpanded(current =>
                      current === session.sessionId ? null : session.sessionId,
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

function SessionRow({
  session,
  expanded,
  onToggle,
}: {
  session: SessionSummary;
  expanded: boolean;
  onToggle: () => void;
}) {
  const detailsId = `session-details-${session.sessionId}`;
  return (
    <>
      <tr className="session-row">
        <td>
          <button
            className="session-title-button"
            type="button"
            aria-expanded={expanded}
            aria-controls={detailsId}
            onClick={onToggle}
          >
            <CaretDown className={expanded ? "expanded" : undefined} size={14} />
            <span>
              <strong title={session.title}>{session.title}</strong>
              {session.childSessionCount > 0 && (
                <small>含 {session.childSessionCount} 个子任务</small>
              )}
            </span>
          </button>
        </td>
        <td
          className="session-secondary"
          title={session.projectPath ?? "未命名项目"}
        >
          {projectName(session.projectPath)}
        </td>
        <td
          className="session-secondary"
          title={session.primaryModel ?? "未知模型"}
        >
          {session.primaryModel ?? "未知模型"}
        </td>
        <td>
          {compactNumber.format(
            totalTokens(session.monthlyTokens ?? session.tokens),
          )}
        </td>
        <td>
          {formatCost(
            session.monthlyEquivalentCostUsd === undefined
              ? session.equivalentCostUsd
              : session.monthlyEquivalentCostUsd,
            session.monthlyUnpricedTokens ?? session.unpricedTokens,
          )}
        </td>
        <td className="session-secondary">{formatActivity(session.lastActiveAt)}</td>
      </tr>
      {expanded && (
        <tr className="session-detail-row" id={detailsId}>
          <td colSpan={6}>
            <dl className="session-breakdown">
              <div>
                <dt>历史总 Token</dt>
                <dd>{fullNumber.format(totalTokens(session.tokens))}</dd>
              </div>
              <div>
                <dt>历史总等效费用</dt>
                <dd>
                  {formatCost(session.equivalentCostUsd, session.unpricedTokens)}
                </dd>
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
                  <dd>
                    {fullNumber.format(session.unpricedTokens)} 未定价 Token
                  </dd>
                </div>
              )}
              <div className="session-path">
                <dt>项目路径</dt>
                <dd>{session.projectPath ?? "未知"}</dd>
              </div>
            </dl>
          </td>
        </tr>
      )}
    </>
  );
}
