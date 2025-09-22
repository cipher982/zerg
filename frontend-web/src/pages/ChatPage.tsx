import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import {
  createThread,
  fetchAgent,
  fetchThreadMessages,
  fetchThreads,
  postThreadMessage,
  runThread,
  Thread,
  ThreadMessage,
} from "../services/api";

function useRequiredNumber(param?: string): number | null {
  if (!param) return null;
  const parsed = Number(param);
  return Number.isFinite(parsed) ? parsed : null;
}

export default function ChatPage() {
  const params = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const agentId = useRequiredNumber(params.agentId);
  const threadIdParam = useRequiredNumber(params.threadId ?? undefined);
  const [selectedThreadId, setSelectedThreadId] = useState<number | null>(threadIdParam);

  useEffect(() => {
    if (threadIdParam !== selectedThreadId) {
      setSelectedThreadId(threadIdParam ?? null);
    }
  }, [threadIdParam, selectedThreadId]);

  const agentQuery = useQuery({
    queryKey: ["agent", agentId],
    queryFn: () => {
      if (agentId == null) {
        return Promise.reject(new Error("Missing agent id"));
      }
      return fetchAgent(agentId);
    },
    enabled: agentId != null,
  });

  const threadsQuery = useQuery({
    queryKey: ["threads", agentId],
    queryFn: () => {
      if (agentId == null) {
        return Promise.reject(new Error("Missing agent id"));
      }
      return fetchThreads(agentId);
    },
    enabled: agentId != null,
  });

  const effectiveThreadId = useMemo(() => {
    if (selectedThreadId != null) {
      return selectedThreadId;
    }
    const threads = threadsQuery.data;
    if (threads && threads.length > 0) {
      return threads[0].id;
    }
    return null;
  }, [selectedThreadId, threadsQuery.data]);

  const messagesQuery = useQuery({
    queryKey: ["thread-messages", effectiveThreadId],
    queryFn: () => {
      if (effectiveThreadId == null) {
        return Promise.resolve<ThreadMessage[]>([]);
      }
      return fetchThreadMessages(effectiveThreadId);
    },
    enabled: effectiveThreadId != null,
  });

  const sendMutation = useMutation({
    mutationFn: async (payload: { threadId: number; content: string }) => {
      await postThreadMessage(payload.threadId, payload.content);
      await runThread(payload.threadId);
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ["thread-messages", variables.threadId] });
    },
  });

  const [draft, setDraft] = useState("");

  const isLoading = agentQuery.isLoading || threadsQuery.isLoading || messagesQuery.isLoading;
  const hasError = agentQuery.isError || threadsQuery.isError || messagesQuery.isError;

  useEffect(() => {
    const origin = window.location.origin.replace("http", "ws");
    const url = new URL("/api/ws", origin);
    const token = localStorage.getItem("zerg_jwt");
    if (token) {
      url.searchParams.set("token", token);
    }
    const ws = new WebSocket(url.toString());
    ws.onmessage = () => {
      queryClient.invalidateQueries({ queryKey: ["threads", agentId] });
      if (effectiveThreadId != null) {
        queryClient.invalidateQueries({ queryKey: ["thread-messages", effectiveThreadId] });
      }
    };
    return () => {
      ws.close();
    };
  }, [agentId, effectiveThreadId, queryClient]);

  useEffect(() => {
    if (agentId != null && effectiveThreadId != null) {
      navigate(`/chat/${agentId}/${effectiveThreadId}`, { replace: true });
    } else if (agentId != null) {
      navigate(`/chat/${agentId}`, { replace: true });
    }
  }, [agentId, effectiveThreadId, navigate]);

  if (agentId == null) {
    return <div>Missing agent context.</div>;
  }

  if (isLoading) {
    return <div>Loading chat…</div>;
  }

  if (hasError) {
    return <div>Unable to load chat view.</div>;
  }

  const threads = threadsQuery.data ?? [];
  const messages = messagesQuery.data ?? [];
  const agent = agentQuery.data;

  const handleSelectThread = (thread: Thread) => {
    setSelectedThreadId(thread.id);
    navigate(`/chat/${agentId}/${thread.id}`, { replace: true });
  };

  const handleSend = (evt: FormEvent) => {
    evt.preventDefault();
    if (!draft.trim() || effectiveThreadId == null) {
      return;
    }
    sendMutation.mutate({ threadId: effectiveThreadId, content: draft });
    setDraft("");
  };

  const handleCreateThread = async () => {
    if (agentId == null) return;
    const title = window.prompt("Thread title", "Untitled Thread") ?? "Untitled Thread";
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      return;
    }
    const thread = await createThread(agentId, trimmedTitle);
    queryClient.invalidateQueries({ queryKey: ["threads", agentId] });
    setSelectedThreadId(thread.id);
    navigate(`/chat/${agentId}/${thread.id}`, { replace: true });
  };

  return (
    <div className="chat-page">
      <header className="chat-header">
        <button onClick={() => navigate("/dashboard")}>← Back</button>
        <div className="agent-meta">
          <h1>{agent?.name ?? "Agent"}</h1>
          {effectiveThreadId != null && (
            <span>
              Thread {effectiveThreadId} ({messages.length} messages)
            </span>
          )}
        </div>
      </header>
      <main className="chat-layout">
        <aside className="thread-list">
          <div className="thread-list__header">
            <h2>Threads</h2>
            <div className="thread-list__actions">
              <button onClick={() => threadsQuery.refetch()}>Refresh</button>
              <button
                className="new-thread-btn"
                data-testid="new-thread-btn"
                onClick={handleCreateThread}
              >
                New Thread
              </button>
            </div>
          </div>
          <ul>
            {threads.map((thread) => (
              <li key={thread.id} className={clsx("thread-row", { selected: thread.id === effectiveThreadId })}>
                <button
                  className={clsx("thread", { active: thread.id === effectiveThreadId })}
                  data-testid={`thread-row-${thread.id}`}
                  onClick={() => handleSelectThread(thread)}
                >
                  {thread.title}
                </button>
              </li>
            ))}
            {threads.length === 0 && <li>No threads yet.</li>}
          </ul>
        </aside>
        <section className="chat-messages">
          <div className="messages-scroll messages-container" data-testid="messages-container">
            {messages.map((msg) => (
              <article
                key={msg.id}
                className={clsx("message", `message--${msg.role}`, {
                  "user-message": msg.role === "user",
                  "assistant-message": msg.role === "assistant",
                })}
                data-testid="chat-message"
              >
                <header>
                  <span>{msg.role}</span>
                  <time>{new Date(msg.created_at).toLocaleTimeString()}</time>
                </header>
                <p>{msg.content}</p>
              </article>
            ))}
            {messages.length === 0 && <p className="empty">No messages yet.</p>}
          </div>
          <form className="chat-compose" onSubmit={handleSend}>
            <input
              type="text"
              value={draft}
              onChange={(evt) => setDraft(evt.target.value)}
              placeholder="Type your message…"
              className="chat-input"
              data-testid="chat-input"
            />
            <button
              type="submit"
              className="send-button"
              disabled={sendMutation.isPending || !draft.trim()}
              data-testid="send-message-btn"
            >
              {sendMutation.isPending ? "Sending…" : "Send"}
            </button>
          </form>
        </section>
      </main>
    </div>
  );
}
