import { ChatCenteredDots } from "@phosphor-icons/react";

export function SessionDetails() {
  return (
    <section className="panel sessions-panel" aria-labelledby="sessions-heading">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">本机记录</span>
          <h2 id="sessions-heading">会话详情</h2>
        </div>
      </div>
      <div className="session-empty">
        <span className="session-empty-icon">
          <ChatCenteredDots size={22} />
        </span>
        <div>
          <strong>本机数据接入后显示</strong>
          <p>届时可查看每个会话的 Token、模型与等效费用。</p>
        </div>
      </div>
    </section>
  );
}
