import { ChatClient, type ChatEvent } from './client';
import type { Message, ChannelSnapshot, UserJoined, UserLeft, MessageEdited } from '$lib/proto/dionysus/chat/v1/message_pb';
import type { UserID } from '$lib/proto/dionysus/user/v1/user_pb';

export interface ChatMessage {
  id: string;
  senderId: string;
  senderName: string;
  text: string;
  timestamp: Date;
  isOwn: boolean;
  repliesTo?: string;
  edited?: boolean;
}

export interface ChatChannel {
  id: string;
  name: string;
  type: 'global' | 'dm';
  participants: string[];
}

export interface KnownUser {
  id: string;
  name: string;
}

function uuidToString(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

function userIdToString(userId: UserID | undefined): string {
  return userId?.value?.value ? uuidToString(userId.value.value) : 'unknown';
}

function userIdToName(userId: UserID | undefined): string {
  const id = userIdToString(userId);
  return id === 'unknown' ? 'Unknown' : `User ${id.slice(0, 8)}`;
}

function createDMChannelId(userId1: string, userId2: string): string {
  const sorted = [userId1, userId2].sort();
  return `dm:${sorted.join(':')}`;
}

export class ChatStore {
  client = new ChatClient();
  messagesByChannel = $state<Record<string, ChatMessage[]>>({});
  channels = $state<Record<string, ChatChannel>>({});
  currentChannelId = $state<string | null>(null);
  currentUserId = $state<string | null>(null);
  currentUserName = $state('Moi');
  isConnected = $state(false);
  connectionError = $state<string | null>(null);
  unsubscribe: (() => void) | null = null;

  // Track known users by ID
  knownUsers = $state<Record<string, KnownUser>>({});

  async connect(userId: Uint8Array, token: string, username: string = 'Moi') {
    this.currentUserName = username;
    this.currentUserId = uuidToString(userId);

    this.unsubscribe = this.client.on((event) => this.handleEvent(event));

    try {
      await this.client.connect(userId, token);
    } catch (e) {
      this.connectionError = e instanceof Error ? e.message : 'Connection failed';
      throw e;
    }
  }

  private handleEvent(event: ChatEvent) {
    switch (event.type) {
      case 'connected':
        this.isConnected = true;
        this.connectionError = null;
        // Join global channel by default
        const globalId = new TextEncoder().encode('global');
        this.client.joinChannel(globalId);
        break;

      case 'disconnected':
        this.isConnected = false;
        break;

      case 'error':
        this.connectionError = event.error.message;
        console.error('[ChatStore] Server error:', event.error.code, event.error.message);
        break;

      case 'snapshot':
        this.handleSnapshot(event.snapshot);
        break;

      case 'message':
        this.handleMessage(event.message);
        break;

      case 'edited':
        this.handleEdited(event.edited);
        break;

      case 'joined':
        this.handleJoined(event.joined);
        break;

      case 'left':
        this.handleLeft(event.left);
        break;

      case 'pong':
        break;
    }
  }

  private handleSnapshot(snapshot: ChannelSnapshot) {
    if (!snapshot.room?.id?.value?.value) {
      return;
    }

    const channelIdBytes = snapshot.room.id.value.value;
    // Check if this is the global channel (bytes for "global")
    const globalBytes = new TextEncoder().encode('global');
    const isGlobal = channelIdBytes.length === globalBytes.length &&
      channelIdBytes.every((b, i) => b === globalBytes[i]);
    const channelId = isGlobal ? 'global' : new TextDecoder().decode(channelIdBytes);
    const channelName = snapshot.room.name || 'Unknown';
    const isDM = channelName.startsWith('dm:');

    // Track users from participants
    for (const participant of snapshot.room.participant || []) {
      const participantId = userIdToString(participant.id);
      const participantName = participant.name || userIdToName(participant.id);
      this.knownUsers[participantId] = { id: participantId, name: participantName };
    }

    this.channels[channelId] = {
      id: channelId,
      name: channelName,
      type: isDM ? 'dm' : 'global',
      participants: snapshot.room.participant?.map(p => p.name || userIdToString(p.id)) || [],
    };

    if (!this.messagesByChannel[channelId]) {
      this.messagesByChannel[channelId] = [];
    }

    const messages = snapshot.recentMessages || [];
    const chatMessages: ChatMessage[] = messages.map(msg => {
      const senderId = userIdToString(msg.senderId);
      // Track known user
      this.knownUsers[senderId] = { id: senderId, name: userIdToName(msg.senderId) };

      return {
        id: uuidToString(msg.id?.value?.value || new Uint8Array()),
        senderId,
        senderName: userIdToName(msg.senderId),
        text: msg.message,
        timestamp: msg.createdAt ? new Date(msg.createdAt.year || 0, (msg.createdAt.month || 1) - 1, msg.createdAt.day || 1, msg.createdAt.hours || 0, msg.createdAt.minutes || 0, msg.createdAt.seconds || 0) : new Date(),
        isOwn: userIdToString(msg.senderId) === this.currentUserId,
        repliesTo: msg.repliesTo?.value?.value ? uuidToString(msg.repliesTo.value.value) : undefined,
      };
    });

    this.messagesByChannel[channelId] = chatMessages;

    if (!this.currentChannelId) {
      this.currentChannelId = channelId;
    }
  }

  private handleMessage(msg: Message) {
    if (!msg.roomId?.value?.value) return;

    const channelIdBytes = msg.roomId.value.value;
    // Check if this is the global channel (bytes for "global")
    const globalBytes = new TextEncoder().encode('global');
    const isGlobal = channelIdBytes.length === globalBytes.length &&
      channelIdBytes.every((b, i) => b === globalBytes[i]);
    const channelId = isGlobal ? 'global' : new TextDecoder().decode(channelIdBytes);

    const senderId = userIdToString(msg.senderId);
    const senderName = userIdToName(msg.senderId);

    // Track known user
    this.knownUsers[senderId] = { id: senderId, name: senderName };

    if (!this.messagesByChannel[channelId]) {
      this.messagesByChannel[channelId] = [];
    }

    const chatMessage: ChatMessage = {
      id: uuidToString(msg.id?.value?.value || new Uint8Array()),
      senderId,
      senderName: userIdToName(msg.senderId),
      text: msg.message,
      timestamp: msg.createdAt ? new Date(msg.createdAt.year || 0, (msg.createdAt.month || 1) - 1, msg.createdAt.day || 1, msg.createdAt.hours || 0, msg.createdAt.minutes || 0, msg.createdAt.seconds || 0) : new Date(),
      isOwn: senderId === this.currentUserId,
      repliesTo: msg.repliesTo?.value?.value ? uuidToString(msg.repliesTo.value.value) : undefined,
    };



    const messages = this.messagesByChannel[channelId] || [];
    messages.push(chatMessage);
    this.messagesByChannel[channelId] = [...messages];
  }

  private handleEdited(edited: MessageEdited) {
    if (!edited.id?.value?.value) return;

    const messageId = uuidToString(edited.id.value.value);

    for (const channelId of Object.keys(this.messagesByChannel)) {
      const messages = this.messagesByChannel[channelId];
      const index = messages.findIndex(m => m.id === messageId);
      if (index !== -1) {
        messages[index] = {
          ...messages[index],
          text: edited.newMessage,
          edited: true,
        };
        this.messagesByChannel[channelId] = [...messages];
        break;
      }
    }
  }

  private handleJoined(joined: UserJoined) {
    if (!joined.channelId?.value?.value || !joined.user) return;

    const channelIdBytes = joined.channelId.value.value;
    const globalBytes = new TextEncoder().encode('global');
    const isGlobal = channelIdBytes.length === globalBytes.length &&
      channelIdBytes.every((b, i) => b === globalBytes[i]);
    const channelId = isGlobal ? 'global' : new TextDecoder().decode(channelIdBytes);

    const channel = this.channels[channelId];
    if (channel) {
      const participants = [...channel.participants, joined.user.name || 'Unknown'];
      this.channels[channelId] = { ...channel, participants };
    }

    // Track the user who joined
    const userId = userIdToString(joined.user.id);
    const userName = joined.user.name || userIdToName(joined.user.id);
    this.knownUsers[userId] = { id: userId, name: userName };
  }

  private handleLeft(left: UserLeft) {
    if (!left.channelId?.value?.value || !left.userId?.value?.value) return;

    const channelIdBytes = left.channelId.value.value;
    const globalBytes = new TextEncoder().encode('global');
    const isGlobal = channelIdBytes.length === globalBytes.length &&
      channelIdBytes.every((b, i) => b === globalBytes[i]);
    const channelId = isGlobal ? 'global' : new TextDecoder().decode(channelIdBytes);
    const userId = uuidToString(left.userId.value.value);
    const channel = this.channels[channelId];
    if (channel) {
      const participants = channel.participants.filter(p => p !== userId);
      this.channels[channelId] = { ...channel, participants };
    }
  }

  currentMessages = $derived.by(() => {
    if (!this.currentChannelId) return [];
    return this.messagesByChannel[this.currentChannelId] || [];
  });

  // Expose currentUserId for components that need it
  // currentUserId is already a $state property

  // DM channels with their participants and channel IDs
  dmChannels = $derived.by(() => {
    const channels: Array<{ channelId: string; name: string; lastMessage: string; timestamp: Date }> = [];
    for (const [channelId, messages] of Object.entries(this.messagesByChannel)) {
      if (channelId.startsWith('dm:') && messages.length > 0) {
        // Find the other participant (not current user)
        const otherMsg = messages.find(m => !m.isOwn);
        if (otherMsg) {
          const lastMsg = messages[messages.length - 1];
          channels.push({
            channelId,
            name: otherMsg.senderName,
            lastMessage: lastMsg.text,
            timestamp: lastMsg.timestamp,
          });
        }
      }
    }
    // Sort by timestamp descending
    return channels.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
  });

  // Known users list for contacts sidebar (excluding self)
  knownUsersList = $derived.by(() => {
    return Object.values(this.knownUsers)
      .filter(user => user.id !== this.currentUserId)
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  sendMessage(text: string) {
    if (!this.currentChannelId || !text.trim()) return;

    const channelId = this.currentChannelId === 'global'
      ? new TextEncoder().encode('global')
      : new TextEncoder().encode(this.currentChannelId);

    this.client.sendMessage(channelId, text.trim());
  }

  joinChannel(channelId: string) {
    const encoded = new TextEncoder().encode(channelId);
    this.client.joinChannel(encoded);
    this.currentChannelId = channelId;
  }

  switchToGlobalChat() {
    this.joinChannel('global');
  }

  startDM(userId: string) {
    // For DM, we create a channel ID using sorted user IDs
    if (!this.currentUserId) return;
    const channelId = createDMChannelId(this.currentUserId, userId);
    this.joinChannel(channelId);
  }

  // Convenience method to start DM by user name (looks up the user ID)
  startDMByName(userName: string) {
    const user = Object.values(this.knownUsers).find(u => u.name === userName);
    if (user) {
      this.startDM(user.id);
    }
  }

  disconnect() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
    this.client.disconnect();
    this.isConnected = false;
  }
}

export const chatStore = new ChatStore();
