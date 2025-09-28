import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import {
  Agent,
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

function resolveWsBase(): string {
  if (typeof window === "undefined") {
    return "";
  }
  return (
    (window as typeof window & { WS_BASE_URL?: string }).WS_BASE_URL ||
    window.location.origin.replace("http", "ws")
  );
}

// Helper functions
function formatTimestamp(timestamp?: string | null): string {
  if (!timestamp) return "";
  try {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return "";
  }
}

function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + "...";
}

// ToolMessage component for rendering collapsible tool call details
interface ToolMessageProps {
  message: ThreadMessage;
}

function ToolMessage({ message }: ToolMessageProps) {
  const toolName = message.tool_name || "tool";
  const toolCallId = message.tool_call_id || "";

  return (
    <details className="disclosure" data-tool-call-id={toolCallId}>
      <summary className="disclosure__summary">
        🛠️ Tool Used: {toolName}
      </summary>
      <div className="disclosure__content">
        <div>
          <div className="tool-detail-row">
            <strong>Tool:</strong> {toolName}
          </div>
          {message.name && (
            <div className="tool-detail-row">
              <strong>Inputs:</strong>
              <pre>{message.name}</pre>
            </div>
          )}
          <div className="tool-detail-row output-row">
            <strong>Output:</strong>
            <pre>{message.content}</pre>
          </div>
        </div>
      </div>
    </details>
  );
}

export default function ChatPage() {
  const params = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const messagesContainerRef = useRef<HTMLDivElement>(null);

  const agentId = useRequiredNumber(params.agentId);
  const threadIdParam = useRequiredNumber(params.threadId ?? undefined);
  const [selectedThreadId, setSelectedThreadId] = useState<number | null>(threadIdParam);
  const [editingThreadId, setEditingThreadId] = useState<number | null>(null);
  const [editingTitle, setEditingTitle] = useState("");

  useEffect(() => {
    if (threadIdParam !== selectedThreadId) {
      setSelectedThreadId(threadIdParam ?? null);
    }
  }, [threadIdParam, selectedThreadId]);

  const agentQuery = useQuery<Agent>({
    queryKey: ["agent", agentId],
    queryFn: () => {
      if (agentId == null) {
        return Promise.reject(new Error("Missing agent id"));
      }
      return fetchAgent(agentId);
    },
    enabled: agentId != null,
  });

  const threadsQuery = useQuery<Thread[]>({
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

  const messagesQuery = useQuery<ThreadMessage[]>({
    queryKey: ["thread-messages", effectiveThreadId],
    queryFn: () => {
      if (effectiveThreadId == null) {
        return Promise.resolve<ThreadMessage[]>([]);
      }
      return fetchThreadMessages(effectiveThreadId);
    },
    enabled: effectiveThreadId != null,
  });

  const sendMutation = useMutation<
    ThreadMessage,
    Error,
    { threadId: number; content: string },
    { previousMessages: ThreadMessage[] | undefined; optimisticId: number }
  >({
    mutationFn: async ({ threadId, content }) => {
      const message = await postThreadMessage(threadId, content);
      await runThread(threadId);
      return message;
    },
    onMutate: async ({ threadId, content }) => {
      await queryClient.cancelQueries({ queryKey: ["thread-messages", threadId] });
      const previousMessages = queryClient.getQueryData<ThreadMessage[]>([
        "thread-messages",
        threadId,
      ]);

      const optimisticId = -Date.now();
      const now = new Date().toISOString();
      const optimisticMessage: ThreadMessage = {
        id: optimisticId,
        thread_id: threadId,
        role: "user",
        content,
        timestamp: now,
        created_at: now,
        processed: true,
      };

      queryClient.setQueryData<ThreadMessage[]>(["thread-messages", threadId], (oldMessages) => {
        if (!oldMessages) {
          return [optimisticMessage];
        }
        return [...oldMessages, optimisticMessage];
      });

      return { previousMessages, optimisticId };
    },
    onError: (_error, variables, context) => {
      if (context?.previousMessages) {
        queryClient.setQueryData(["thread-messages", variables.threadId], context.previousMessages);
      }
    },
    onSuccess: (data, variables, context) => {
      queryClient.setQueryData<ThreadMessage[]>(["thread-messages", variables.threadId], (current) => {
        if (!current) {
          return [data];
        }
        if (context) {
          return current.map((message) =>
            message.id === context.optimisticId ? data : message
          );
        }
        return [...current, data];
      });
    },
    onSettled: (_data, _error, variables) => {
      queryClient.invalidateQueries({ queryKey: ["thread-messages", variables.threadId] });
    },
  });

  const [draft, setDraft] = useState("");

  const isLoading = agentQuery.isLoading || threadsQuery.isLoading || messagesQuery.isLoading;
  const hasError = agentQuery.isError || threadsQuery.isError || messagesQuery.isError;

  useEffect(() => {
    if (typeof performance === "undefined") {
      return;
    }

    const legacyNav = (performance as Performance & { navigation?: PerformanceNavigation }).navigation;
    if (legacyNav && legacyNav.type === legacyNav.TYPE_RELOAD) {
      navigate("/dashboard", { replace: true });
      return;
    }

    if (typeof performance.getEntriesByType === "function") {
      const entries = performance.getEntriesByType("navigation");
      const latest = entries[entries.length - 1] as PerformanceNavigationTiming | undefined;
      if (latest?.type === "reload") {
        navigate("/dashboard", { replace: true });
      }
    }
  }, [navigate]);

  useEffect(() => {
    const base = resolveWsBase();
    const url = new URL("/api/ws", base);
    const token = localStorage.getItem("zerg_jwt");
    if (token) {
      url.searchParams.set("token", token);
    }
    const workerId = window.__TEST_WORKER_ID__;
    if (workerId !== undefined) {
      url.searchParams.set("worker", String(workerId));
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

  const threads = useMemo(() => {
    const list = threadsQuery.data ?? [];
    // Sort threads by updated_at (newest first), falling back to created_at
    return [...list].sort((a, b) => {
      const aTime = a.updated_at || a.created_at;
      const bTime = b.updated_at || b.created_at;
      return bTime.localeCompare(aTime);
    });
  }, [threadsQuery.data]);

  const messages = useMemo(() => {
    const list = messagesQuery.data ?? [];
    // Sort messages by ID for stable chronological order
    return [...list].sort((a, b) => a.id - b.id);
  }, [messagesQuery.data]);

  const agent = agentQuery.data;

  // Group tool messages by parent_id for rendering under assistant messages
  const toolMessagesByParent = useMemo(() => {
    const map = new Map<number, ThreadMessage[]>();
    messages
      .filter(m => m.role === "tool" && m.parent_id != null)
      .forEach(msg => {
        const list = map.get(msg.parent_id!) || [];
        list.push(msg);
        map.set(msg.parent_id!, list);
      });
    return map;
  }, [messages]);

  // Get orphaned tool messages (no parent_id)
  const orphanedToolMessages = useMemo(() => {
    return messages.filter(m => m.role === "tool" && m.parent_id == null);
  }, [messages]);

  const handleSelectThread = (thread: Thread) => {
    setSelectedThreadId(thread.id);
    navigate(`/chat/${agentId}/${thread.id}`, { replace: true });
  };

  const handleEditThreadTitle = (thread: Thread, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingThreadId(thread.id);
    setEditingTitle(thread.title);
  };

  const handleSaveThreadTitle = async (threadId: number) => {
    if (!editingTitle.trim()) {
      setEditingThreadId(null);
      return;
    }

    // TODO: Call API to update thread title
    // For now, just update locally
    queryClient.setQueryData<Thread[]>(["threads", agentId], (old) => {
      if (!old) return old;
      return old.map(t => t.id === threadId ? { ...t, title: editingTitle } : t);
    });

    setEditingThreadId(null);
  };

  const handleCancelEdit = () => {
    setEditingThreadId(null);
    setEditingTitle("");
  };

  const ensureActiveThread = useCallback(async () => {
    if (effectiveThreadId != null) {
      return effectiveThreadId;
    }

    if (threads.length > 0) {
      const firstThreadId = threads[0].id;
      setSelectedThreadId(firstThreadId);
      return firstThreadId;
    }

    if (agentId == null) {
      return null;
    }

    try {
      const thread = await createThread(agentId, "Primary Thread");
      await queryClient.invalidateQueries({ queryKey: ["threads", agentId] });
      setSelectedThreadId(thread.id);
      return thread.id;
    } catch (error) {
      console.error("Failed to create thread", error);
      return null;
    }
  }, [agentId, effectiveThreadId, queryClient, threads]);

  const handleSend = async (evt: FormEvent) => {
    evt.preventDefault();
    const trimmed = draft.trim();
    if (!trimmed) {
      return;
    }
    const threadId = await ensureActiveThread();
    if (threadId == null) {
      return;
    }
    setDraft("");
    try {
      await sendMutation.mutateAsync({ threadId, content: trimmed });
    } catch (error) {
      console.error("Failed to send message", error);
    }
  };

  // Scroll to bottom when messages change
  useEffect(() => {
    if (messagesContainerRef.current) {
      messagesContainerRef.current.scrollTop = messagesContainerRef.current.scrollHeight;
    }
  }, [messages]);

  if (agentId == null) {
    return <div>Missing agent context.</div>;
  }

  if (isLoading) {
    return <div>Loading chat…</div>;
  }

  if (hasError) {
    return <div>Unable to load chat view.</div>;
  }

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
    <div id="chat-view-container" className="chat-view-container">
      <header className="chat-header">
        <button
          type="button"
          className="back-button"
          onClick={() => navigate("/dashboard")}
        >
          ←
        </button>
        {agent?.id != null && (
          <button
            type="button"
            data-testid={`chat-agent-${agent.id}`}
            onClick={() => {
              if (effectiveThreadId != null) {
                navigate(`/chat/${agent.id}/${effectiveThreadId}`, { replace: true });
              } else {
                navigate(`/chat/${agent.id}`, { replace: true });
              }
            }}
            aria-hidden="true"
            tabIndex={-1}
            style={{
              position: "absolute",
              width: 1,
              height: 1,
              opacity: 0,
              pointerEvents: "auto",
              overflow: "hidden",
            }}
          >
            {agent.name}
          </button>
        )}
        <div className="agent-info">
          <div className="agent-name">{agent?.name ?? "Agent"}</div>
          <div>
            <span className="thread-title-label">Thread: </span>
            <span className="thread-title-text">
              {effectiveThreadId != null ? `#${effectiveThreadId}` : "None"}
            </span>
          </div>
        </div>
      </header>

      <div className="chat-body">
        <aside className="thread-sidebar">
          <div className="sidebar-header">
            <h3>Threads</h3>
            <button
              className="new-thread-btn"
              data-testid="new-thread-btn"
              onClick={handleCreateThread}
            >
              New Thread
            </button>
          </div>
          <div className="thread-list">
            {threads.map((thread) => {
              const threadMessages = messages.filter(m => m.thread_id === thread.id);
              const lastMessage = threadMessages[threadMessages.length - 1];
              const messagePreview = lastMessage
                ? truncateText(lastMessage.content, 50)
                : "No messages";

              return (
                <div
                  key={thread.id}
                  className={clsx("thread-item", { selected: thread.id === effectiveThreadId })}
                  data-testid={`thread-row-${thread.id}`}
                  data-id={thread.id}
                  data-thread-id={thread.id}
                  role="button"
                  tabIndex={0}
                  onClick={() => handleSelectThread(thread)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      handleSelectThread(thread);
                    }
                  }}
                >
                  {editingThreadId === thread.id ? (
                    <div className="thread-edit-form" onClick={(e) => e.stopPropagation()}>
                      <input
                        type="text"
                        value={editingTitle}
                        onChange={(e) => setEditingTitle(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            handleSaveThreadTitle(thread.id);
                          } else if (e.key === "Escape") {
                            handleCancelEdit();
                          }
                        }}
                        autoFocus
                        className="thread-title-input"
                      />
                    </div>
                  ) : (
                    <>
                      <div className="thread-item-title">{thread.title}</div>
                      <div className="thread-item-time">
                        {formatTimestamp(thread.updated_at || thread.created_at)}
                      </div>
                      <div
                        className="thread-edit-button"
                        onClick={(e) => handleEditThreadTitle(thread, e)}
                        aria-label="Edit thread title"
                      >
                        ✎
                      </div>
                      <div className="thread-item-preview">{messagePreview}</div>
                    </>
                  )}
                </div>
              );
            })}
            {threads.length === 0 && (
              <div className="thread-list-empty">No threads found</div>
            )}
          </div>
        </aside>

        <section className="conversation-area">
          <div className="messages-container" data-testid="messages-container" ref={messagesContainerRef}>
            {messages
              .filter(msg => msg.role !== "system" && msg.role !== "tool")
              .map((msg, index) => {
                const isLastUserMessage = msg.role === "user" && index === messages.length - 1;
                const toolMessages = toolMessagesByParent.get(msg.id);

                // Skip rendering empty assistant messages (they only have tool calls)
                if (msg.role === "assistant" && msg.content.trim() === "") {
                  return (
                    <div key={msg.id}>
                      {toolMessages?.map(toolMsg => (
                        <ToolMessage key={toolMsg.id} message={toolMsg} />
                      ))}
                    </div>
                  );
                }

                return (
                  <div key={msg.id}>
                    <div className="chat-row">
                      <article
                        className={clsx("message", {
                          "user-message": msg.role === "user",
                          "assistant-message": msg.role === "assistant",
                        })}
                        data-testid={isLastUserMessage ? "chat-message" : undefined}
                        data-role={`chat-message-${msg.role}`}
                      >
                        <div className="message-content preserve-whitespace">{msg.content}</div>
                        <div className="message-time">{formatTimestamp(msg.timestamp)}</div>
                      </article>
                    </div>
                    {msg.role === "assistant" && toolMessages?.map(toolMsg => (
                      <ToolMessage key={toolMsg.id} message={toolMsg} />
                    ))}
                  </div>
                );
              })}
            {orphanedToolMessages.map(toolMsg => (
              <ToolMessage key={toolMsg.id} message={toolMsg} />
            ))}
            {messages.length === 0 && (
              <p className="thread-list-empty">No messages yet.</p>
            )}
          </div>
        </section>
      </div>

      <form className="chat-input-area" onSubmit={handleSend}>
        <input
          type="text"
          value={draft}
          onChange={(evt) => setDraft(evt.target.value)}
          placeholder={effectiveThreadId ? "Type your message..." : "Select a thread to start chatting"}
          className="chat-input"
          data-testid="chat-input"
          disabled={!effectiveThreadId}
        />
        <button
          type="submit"
          className={clsx("send-button", { disabled: !effectiveThreadId })}
          disabled={sendMutation.isPending || !draft.trim() || !effectiveThreadId}
          data-testid="send-message-btn"
        >
          {sendMutation.isPending ? "Sending…" : "Send"}
        </button>
      </form>
    </div>
  );
}
