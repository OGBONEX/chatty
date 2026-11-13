import { FormEvent, useEffect, useState } from 'react';
import { Bell, Hash, Headphones, MessageCircle, Plus, Search, Settings, Shield, Users, Volume2, ChevronDown, Send, Download, X, LogOut } from 'lucide-react';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://127.0.0.1:8080';
type Account = { id: string; email: string; username: string; display_name?: string | null; bio: string; avatar_url?: string | null; banner_url?: string | null; status_text?: string | null };
type AuthResult = { token: string; expires_at: string; account: Account };
type Message = { name: string; text: string; time: string };
type Friend = { id: string; username: string; display_name: string | null };
type FriendRequest = { id: string; sender_id: string; sender_username: string; recipient_id: string; recipient_username: string; status: string; created_at: string };
type DirectMessage = { id: string; sender_id: string; recipient_id: string; content: string; created_at: string };
type Preferences = { dark:boolean; compact:boolean; notifications:boolean; accent:string; reduceMotion:boolean; largerText:boolean; enterToSend:boolean; showTimestamps:boolean };
const DEFAULT_PREFERENCES: Preferences = { dark:true, compact:false, notifications:true, accent:'#a56af5', reduceMotion:false, largerText:false, enterToSend:true, showTimestamps:true };
function readPreferences(): Preferences { try { return { ...DEFAULT_PREFERENCES, ...JSON.parse(localStorage.getItem('chatty_preferences') ?? '{}') }; } catch { return DEFAULT_PREFERENCES; } }
const seed: Message[] = [
  { name: 'Nova', time: '10:42', text: 'Welcome to the Chatty preview! This is the start of a new community.' },
  { name: 'PixelPilot', time: '10:44', text: 'The interface is taking shape. More features will arrive as the backend grows.' },
];

export default function App() {
  const [account, setAccount] = useState<Account | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [authLoading, setAuthLoading] = useState(true);
  const [page,setPage]=useState<'chat'|'settings'|'friends'|'dm'>('chat');
  const [friends,setFriends]=useState<Friend[]>([]);
  const [friendRequests,setFriendRequests]=useState<FriendRequest[]>([]);
  const [selectedFriend,setSelectedFriend]=useState<Friend|null>(null);
  const [dmMessages,setDmMessages]=useState<DirectMessage[]>([]);
  const [socialError,setSocialError]=useState('');
  const [friendUsername,setFriendUsername]=useState('');
  const [channel,setChannel]=useState('general');
  const [messages,setMessages]=useState(seed);
  const [draft,setDraft]=useState('');
  const [search,setSearch]=useState('');
  const [preferences,setPreferences]=useState<Preferences>(readPreferences);
  const {dark,compact,notifications,accent,reduceMotion,largerText,enterToSend,showTimestamps}=preferences;
  const setDark=(value:boolean)=>setPreferences(current=>({...current,dark:value}));
  const setCompact=(value:boolean)=>setPreferences(current=>({...current,compact:value}));
  const setNotifications=(value:boolean)=>setPreferences(current=>({...current,notifications:value}));
  const updatePreference=<K extends keyof Preferences>(key:K,value:Preferences[K])=>setPreferences(current=>({...current,[key]:value}));
  useEffect(()=>{localStorage.setItem('chatty_preferences',JSON.stringify(preferences));},[preferences]);
  const [updateOpen,setUpdateOpen]=useState(false);
  const [tab,setTab]=useState('My Account');
  const [authMode,setAuthMode]=useState<'login'|'register'>('login');
  const [authError,setAuthError]=useState('');
  const [authBusy,setAuthBusy]=useState(false);
  const [profileName,setProfileName]=useState('');
  const [profileBio,setProfileBio]=useState('');
  const [profileAvatarUrl,setProfileAvatarUrl]=useState('');
  const [profileBannerUrl,setProfileBannerUrl]=useState('');
  const [profileStatusText,setProfileStatusText]=useState('');
  const [profileStatus,setProfileStatus]=useState('');
  const [profileBusy,setProfileBusy]=useState(false);

  async function authenticate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setAuthError('');
    setAuthBusy(true);
    const data = new FormData(event.currentTarget);
    const payload = authMode === 'register'
      ? { email: String(data.get('email') ?? ''), username: String(data.get('username') ?? ''), password: String(data.get('password') ?? '') }
      : { login: String(data.get('login') ?? ''), password: String(data.get('password') ?? '') };
    try {
      const response = await fetch(`${API}/auth/${authMode}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const result = await response.json();
      if (!response.ok) throw new Error(result.error ?? 'Authentication failed');
      const auth = result as AuthResult;
      sessionStorage.setItem('chatty_token', auth.token);
      setToken(auth.token);
      setAccount(auth.account);
      setProfileName(auth.account.display_name ?? '');
      setProfileBio(auth.account.bio ?? '');
      setProfileAvatarUrl(auth.account.avatar_url ?? '');
      setProfileBannerUrl(auth.account.banner_url ?? '');
      setProfileStatusText(auth.account.status_text ?? '');
      setProfileStatus('');
      await refreshSocial(auth.token);
    } catch (error) {
      setAuthError(error instanceof Error ? error.message.replaceAll('_', ' ') : 'Could not connect to Chatty. Check that the API is running.');
    } finally {
      setAuthBusy(false);
    }
  }

  async function saveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token) return;
    setProfileBusy(true);
    setProfileStatus('');
    try {
      const response = await fetch(`${API}/auth/profile`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ display_name: profileName, bio: profileBio, avatar_url: profileAvatarUrl, banner_url: profileBannerUrl, status_text: profileStatusText }),
      });
      const result = await response.json();
      if (!response.ok) throw new Error(result.error ?? 'Profile update failed');
      setAccount(current => current ? { ...current, display_name: result.display_name, bio: result.bio, avatar_url: result.avatar_url, banner_url: result.banner_url, status_text: result.status_text } : current);
      setProfileStatus('Profile saved.');
    } catch (error) {
      setProfileStatus(error instanceof Error ? error.message.replaceAll('_', ' ') : 'Could not save profile.');
    } finally {
      setProfileBusy(false);
    }
  }


  async function refreshSocial(authToken = token) {
    if (!authToken) return;
    try {
      const headers = { Authorization: `Bearer ${authToken}` };
      const [friendResponse, requestResponse] = await Promise.all([
        fetch(`${API}/friends`, { headers }),
        fetch(`${API}/friends/requests`, { headers }),
      ]);
      if (!friendResponse.ok || !requestResponse.ok) throw new Error('Could not load friends');
      setFriends(await friendResponse.json());
      setFriendRequests(await requestResponse.json());
      setSocialError('');
    } catch (error) { setSocialError(error instanceof Error ? error.message : 'Could not load friends'); }
  }
  useEffect(() => {
    const savedToken = sessionStorage.getItem('chatty_token');
    if (!savedToken) { setAuthLoading(false); return; }
    let cancelled = false;
    async function restoreSession() {
      try {
        const response = await fetch(`${API}/auth/me`, { headers: { Authorization: `Bearer ${savedToken}` } });
        if (!response.ok) throw new Error('session_expired');
        const restored = await response.json() as Account;
        if (cancelled) return;
        setAccount(restored);
        setToken(savedToken);
        setProfileName(restored.display_name ?? '');
        setProfileBio(restored.bio ?? '');
        setProfileAvatarUrl(restored.avatar_url ?? '');
        setProfileBannerUrl(restored.banner_url ?? '');
        setProfileStatusText(restored.status_text ?? '');
        await refreshSocial(savedToken);
      } catch {
        sessionStorage.removeItem('chatty_token');
      } finally {
        if (!cancelled) setAuthLoading(false);
      }
    }
    void restoreSession();
    return () => { cancelled = true; };
  }, []);

  async function addFriend(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); if (!token) return; setSocialError('');
    try {
      const response = await fetch(`${API}/friends/requests`, { method:'POST', headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`}, body:JSON.stringify({username:friendUsername}) });
      const result=await response.json(); if(!response.ok) throw new Error(result.error??'Could not send request');
      setFriendUsername(''); await refreshSocial();
    } catch(error) { setSocialError(error instanceof Error?error.message.replaceAll('_',' '):'Could not send request'); }
  }
  async function answerFriendRequest(id:string, action:'accept'|'decline') {
    if(!token)return;
    try {
      const response=await fetch(`${API}/friends/requests/${id}/${action}`,{method:'POST',headers:{Authorization:`Bearer ${token}`}});
      if(!response.ok){const result=await response.json();throw new Error(result.error??'Could not update request');}
      await refreshSocial();
    } catch(error){setSocialError(error instanceof Error?error.message.replaceAll('_',' '):'Could not update request');}
  }
  async function openDirectMessage(friend:Friend) {
    if(!token)return; setSocialError('');
    try {
      const response=await fetch(`${API}/dm/${friend.id}/messages`,{headers:{Authorization:`Bearer ${token}`}});
      const result=await response.json(); if(!response.ok)throw new Error(result.error??'Could not load messages');
      setSelectedFriend(friend);setDmMessages(result);setPage('dm');
    } catch(error){setSocialError(error instanceof Error?error.message.replaceAll('_',' '):'Could not load messages');setPage('friends');}
  }
  async function sendDirectMessage(event:FormEvent<HTMLFormElement>,content:string) {
    event.preventDefault();if(!token||!selectedFriend||!content.trim())return;
    const response=await fetch(`${API}/dm/${selectedFriend.id}/messages`,{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`},body:JSON.stringify({content})});
    const result=await response.json();if(!response.ok){setSocialError(result.error?.replaceAll('_',' ')??'Could not send message');return;}
    setDmMessages(current=>current.some(message=>message.id===result.id)?current:[...current,result]);
  }
  useEffect(()=>{
    if(!token||!selectedFriend||page!=='dm')return;
    const activeFriendId = selectedFriend.id;
    let socket:WebSocket|null=null;let cancelled=false;
    async function connect(){
      try{
        const response=await fetch(`${API}/auth/ws-ticket`,{method:'POST',headers:{Authorization:`Bearer ${token}`}});
        if(!response.ok)return;const {ticket}=await response.json();if(cancelled)return;
        const endpoint=new URL('/ws',API);endpoint.protocol=endpoint.protocol==='https:'?'wss:':'ws:';endpoint.searchParams.set('ticket',ticket);
        socket=new WebSocket(endpoint.toString());
        socket.onmessage=event=>{try{
          const payload=JSON.parse(event.data);const message=payload.message as DirectMessage;
          if(payload.event==='message.created'&&((message.sender_id===account?.id&&message.recipient_id===activeFriendId)||(message.sender_id===activeFriendId&&message.recipient_id===account?.id))){
            setDmMessages(current=>current.some(item=>item.id===message.id)?current:[...current,message]);
          }
        }catch{/* Ignore malformed events. */}};
        socket.onclose=()=>{if(!cancelled)setSocialError('Live connection closed. Reopen the conversation to reconnect.');};
      }catch{setSocialError('Live connection unavailable.');}
    }
    void connect();return()=>{cancelled=true;socket?.close();};
  },[token,selectedFriend?.id,page,account?.id]);

  async function logout() {
    if (token) {
      try { await fetch(`${API}/auth/logout`, { method: 'POST', headers: { Authorization: `Bearer ${token}` } }); } catch { /* local sign-out still proceeds */ }
    }
    sessionStorage.removeItem('chatty_token');
    setToken(null);
    setAccount(null);
    setProfileName('');
    setProfileBio('');
    setProfileAvatarUrl('');
    setProfileBannerUrl('');
    setProfileStatusText('');
    setPage('chat');
  }

  function send(e:FormEvent){e.preventDefault();const text=draft.trim();if(!text)return;setMessages(v=>[...v,{name:account?.display_name || account?.username || 'You',time:new Date().toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'}),text}]);setDraft('');}
  const shown=messages.filter(m=>(m.text+m.name).toLowerCase().includes(search.toLowerCase()));

  if (authLoading) return <main className="auth-screen"><div className="auth-card"><div className="auth-logo">C</div><h1>Loading Chatty…</h1><p>Restoring your session.</p></div></main>;
  if (!account || !token) return <AuthScreen mode={authMode} setMode={setAuthMode} submit={authenticate} busy={authBusy} error={authError}/>;

  return <div className={(dark?'app dark':'app light')+(reduceMotion?' reduce-motion':'')+(largerText?' larger-text':'')} style={{ ['--accent' as any]: accent }}>
    <aside className="servers"><button className="server-logo active">C</button><div className="server-divider"/><button className="server-logo"><Users/></button><button className="server-logo add"><Plus/></button></aside>
    <aside className="sidebar">
      <button className="workspace"><span className="brand-mark">C</span><span><b>Chatty</b><small>Community space</small></span><ChevronDown size={17}/></button>
      <button className={'nav-item '+(page==='friends'||page==='dm'?'selected':'')} onClick={()=>{setPage('friends');void refreshSocial()}}><MessageCircle size={18}/> Direct Messages</button>
      {friends.map(friend=><button key={friend.id} className="channel" onClick={()=>void openDirectMessage(friend)}><MessageCircle size={15}/>{friend.display_name||friend.username}</button>)}
      <div className="section-label">TEXT CHANNELS <Plus size={15}/></div>
      {['general','announcements','media','off-topic'].map(c=><button key={c} className={'channel '+(channel===c?'selected':'')} onClick={()=>{setChannel(c);setPage('chat')}}><Hash size={17}/>{c}</button>)}
      <div className="section-label voice-label">VOICE CHANNELS <Plus size={15}/></div>
      <button className="channel"><Volume2 size={17}/> Lounge</button><button className="channel"><Headphones size={17}/> Gaming</button>
      <div className="sidebar-bottom"><div className="user-avatar">{account.avatar_url?<img src={account.avatar_url} alt="" />:(account.display_name || account.username)[0].toUpperCase()}</div><span className="user-meta"><b>{account.display_name || account.username}</b><small>Signed in</small></span><button className="icon-button" title="Sign out" onClick={logout}><LogOut size={17}/></button><button className="icon-button" title="Settings" onClick={()=>setPage('settings')}><Settings size={18}/></button></div>
    </aside>
    <main className="main">
      <header className="topbar"><div className="channel-title"><Hash size={20}/><b>{page==='settings'?'User Settings':channel}</b></div><div className="top-actions"><label className="search"><Search size={16}/><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search local messages"/></label><button className="icon-button" title="Toggle notifications" onClick={()=>setNotifications(v=>!v)}><Bell size={19}/></button><button className="update-button" onClick={()=>setUpdateOpen(v=>!v)}><Download size={17}/> Updates</button><button className="icon-button" title="Settings" onClick={()=>setPage(page==='settings'?'chat':'settings')}><Settings size={19}/></button></div></header>
      {updateOpen&&<div className="update-popover"><button className="close" onClick={()=>setUpdateOpen(false)}><X size={16}/></button><b>Preview build</b><p>Chatty v0.1.0</p><small>Signed update delivery will be connected for desktop releases in a later milestone.</small></div>}
      {page==='friends'?<FriendsPanel account={account} friends={friends} requests={friendRequests} username={friendUsername} setUsername={setFriendUsername} error={socialError} addFriend={addFriend} answer={answerFriendRequest} openDM={openDirectMessage}/>:page==='dm'&&selectedFriend?<DirectMessagePanel account={account} friend={selectedFriend} messages={dmMessages} error={socialError} send={sendDirectMessage} enterToSend={enterToSend} back={()=>setPage('friends')}/>:page==='settings'?<section className="settings-layout"><nav className="settings-nav"><h2>Settings</h2>{['My Account','Privacy & Safety','Appearance','Notifications','Voice & Video','Text & Messages','Keybinds','Activity','Language & Region','Accessibility','Storage','Advanced','Updates'].map(t=><button key={t} className={tab===t?'setting-active':''} onClick={()=>setTab(t)}>{t}</button>)}</nav><div className="settings-content"><h1>{tab}</h1><p className="muted">Manage your Chatty experience.</p>{tab==='My Account'?<><Setting title="Username" desc={account.username}/><Setting title="Email" desc={account.email}/><Setting title="Account ID" desc={account.id}/><form className="profile-form" onSubmit={saveProfile}><h2>Edit profile</h2>
          <div className="profile-preview" style={profileBannerUrl?{backgroundImage:`linear-gradient(90deg,#17131f88,#17131f22),url("${profileBannerUrl}")`}:undefined}>
            <div className="profile-preview-avatar">{profileAvatarUrl?<img src={profileAvatarUrl} alt="Profile avatar preview"/>:(profileName||account.username).slice(0,1).toUpperCase()}</div>
            <div><b>{profileName||account.username}</b><small>@{account.username}</small>{profileStatusText&&<span>{profileStatusText}</span>}</div>
          </div>
          <label>Display name<input value={profileName} onChange={event=>setProfileName(event.target.value)} maxLength={32} placeholder="How others see you"/></label>
          <label>Profile picture URL<input type="url" value={profileAvatarUrl} onChange={event=>setProfileAvatarUrl(event.target.value)} maxLength={2048} placeholder="https://example.com/avatar.png"/></label>
          <label>Profile banner URL<input type="url" value={profileBannerUrl} onChange={event=>setProfileBannerUrl(event.target.value)} maxLength={2048} placeholder="https://example.com/banner.png"/></label>
          <label>Status<input value={profileStatusText} onChange={event=>setProfileStatusText(event.target.value)} maxLength={80} placeholder="What are you up to?"/></label>
          <label>About me<textarea value={profileBio} onChange={event=>setProfileBio(event.target.value)} maxLength={280} rows={4} placeholder="Tell people a little about yourself"/></label>
          <div className="profile-form-footer"><small>{profileBio.length}/280 · {profileStatusText.length}/80</small><button className="auth-submit" disabled={profileBusy}>{profileBusy?'Saving…':'Save profile'}</button></div>{profileStatus&&<p className="profile-status" role="status">{profileStatus}</p>}</form></>:tab==='Appearance'?<><Setting title="Dark theme" desc="Switch between dark and light appearance."/><Toggle checked={dark} change={setDark}/><Setting title="Compact messages" desc="Use tighter spacing in message lists."/><Toggle checked={compact} change={setCompact}/><Setting title="Accent colour" desc="Choose the colour used by buttons and active controls."/><div className="accent-options">{['#a56af5','#3b82f6','#16a085','#e67e22','#e84393','#e5b93d'].map(color=><button type="button" key={color} aria-label={'Set accent '+color} aria-pressed={accent===color} className={accent===color?'accent-choice chosen':'accent-choice'} style={{backgroundColor:color}} onClick={()=>updatePreference('accent',color)}/>)}</div></>:tab==='Notifications'?<><Setting title="In-app notifications" desc="Enable notification indicators and alerts inside Chatty."/><Toggle checked={notifications} change={setNotifications}/><Setting title="Browser notifications" desc={typeof Notification==='undefined'?'This browser does not support desktop notifications.':Notification.permission==='granted'?'Browser notifications are allowed.':Notification.permission==='denied'?'Notifications are blocked in browser permissions.':'Allow Chatty to show browser notifications.'}/><button className="secondary settings-action" type="button" disabled={typeof Notification==='undefined'||Notification.permission==='denied'} onClick={()=>{if(typeof Notification!=='undefined')void Notification.requestPermission();}}>Enable browser notifications</button></>:tab==='Text & Messages'?<><Setting title="Enter to send" desc="Press Enter to send; Shift+Enter adds a new line."/><Toggle checked={enterToSend} change={value=>updatePreference('enterToSend',value)}/><Setting title="Show message timestamps" desc="Display the time next to messages."/><Toggle checked={showTimestamps} change={value=>updatePreference('showTimestamps',value)}/></>:tab==='Accessibility'?<><Setting title="Reduce motion" desc="Disable interface transitions and animations."/><Toggle checked={reduceMotion} change={value=>updatePreference('reduceMotion',value)}/><Setting title="Larger text" desc="Increase text size across Chatty."/><Toggle checked={largerText} change={value=>updatePreference('largerText',value)}/></>:tab==='Privacy & Safety'?<><Setting title="Account security" desc="Your current session can be ended from the account menu. Server-enforced privacy controls are being added with the API."/><div className="notice"><Shield size={18}/> Chatty will not show privacy controls that are not enforced by the server.</div></>:tab==='Voice & Video'?<><Setting title="Voice and video" desc="Voice rooms, device selection and video calls are not connected yet."/><div className="notice"><Headphones size={18}/> This section will become available when real-time voice infrastructure is added.</div></>:tab==='Keybinds'?<><Setting title="Send message" desc={enterToSend?'Enter':'Ctrl + Enter'}/><Setting title="New line" desc={enterToSend?'Shift + Enter':'Enter'}/><Setting title="Open settings" desc="Use the settings button in the top bar."/></>:tab==='Language & Region'?<Setting title="Display language" desc="English (United States) · more translations are planned."/>:tab==='Activity'?<Setting title="Activity status" desc="Rich presence and activity sharing are not available yet."/>:tab==='Storage'?<><Setting title="Local preferences" desc="Appearance and composer preferences are stored in this browser."/><button className="secondary settings-action" type="button" onClick={()=>{localStorage.removeItem('chatty_preferences');setPreferences(DEFAULT_PREFERENCES);}}>Reset local preferences</button></>:tab==='Advanced'?<><Setting title="Reset settings" desc="Restore default local appearance and messaging preferences."/><button className="secondary settings-action" type="button" onClick={()=>setPreferences(DEFAULT_PREFERENCES)}>Restore defaults</button></>:tab==='Updates'?<Setting title="Application updates" desc="Signed update checks will be connected to platform releases when desktop builds are available."/>:<Setting title="Settings section" desc="This setting is not available yet."/></div></section>:<section className="chat-area"><div className="welcome"><div className="welcome-icon"><Hash/></div><h1>Welcome to #{channel}</h1><p>This is the beginning of the #{channel} channel.</p></div><div className="messages">{shown.map((m,i)=><article className={'message '+(compact?'compact':'')} key={i}><div className="avatar">{m.name[0]}</div><div><div className="message-meta"><b>{m.name}</b>{showTimestamps&&<time>{m.time}</time>}</div><MessageBody content={m.text}/></div></article>)}</div><RichComposer value={draft} setValue={setDraft} onSubmit={send} placeholder={'Message #'+channel} enterToSend={enterToSend}/></section>}
      <footer className="statusbar"><span><i/> Signed in · Direct messages sync through PostgreSQL</span><span>Chatty · 0.1.0</span></footer>
    </main>
  </div>;
}

const EMOJI_GROUPS = [
  { label: 'Faces', items: ['😀','😃','😄','😁','😆','😂','🤣','🥹','😊','😍','🥰','😘','😎','🤔','🙄','😭','😡','🥳','🤯','😴','🤗','🫡'] },
  { label: 'Hands', items: ['👋','👍','👎','👏','🙌','🤝','🙏','💪','✌️','🤞','🫶','❤️'] },
  { label: 'Things', items: ['🔥','✨','🎉','💯','⭐','🚀','🎮','🎧','💜','💀','👀','💬'] },
];

function allowedGifUrl(value: string) {
  try {
    const url = new URL(value.trim());
    return url.protocol === 'https:' && ['media.tenor.com','media.giphy.com','i.giphy.com'].includes(url.hostname);
  } catch { return false; }
}

function MessageBody({content}:{content:string}) {
  const gif = content.match(/^GIF:(https:\/\/(?:media\.tenor\.com|media\.giphy\.com|i\.giphy\.com)\/\S+)$/i)?.[1]
    ?? (allowedGifUrl(content) ? content : null);
  if (gif && allowedGifUrl(gif)) return <div className="message-gif"><img src={gif} alt="Shared GIF" loading="lazy"/><a href={gif} target="_blank" rel="noreferrer">Open GIF</a></div>;
  const parts = content.split(/(https?:\/\/[^\s]+)/g);
  return <p className="message-body">{parts.map((part,index)=>/^https?:\/\//.test(part)
    ? <a key={index} href={part} target="_blank" rel="noreferrer">{part}</a>
    : <span key={index}>{part}</span>)}</p>;
}

function RichComposer({value,setValue,onSubmit,placeholder,enterToSend=true}:{value:string;setValue:(value:string)=>void;onSubmit:(event:FormEvent<HTMLFormElement>)=>void;placeholder:string;enterToSend?:boolean}) {
  const [emojiOpen,setEmojiOpen]=useState(false);
  const [gifOpen,setGifOpen]=useState(false);
  const [gifQuery,setGifQuery]=useState('');
  const [gifUrl,setGifUrl]=useState('');
  const [gifError,setGifError]=useState('');
  function insertEmoji(emoji:string) { setValue(value+emoji);setEmojiOpen(false); }
  function insertGif() {
    if(!allowedGifUrl(gifUrl)){setGifError('Paste a direct HTTPS GIF link from Tenor or GIPHY.');return;}
    setValue((value?value+' ':'')+'GIF:'+gifUrl.trim());
    setGifUrl('');setGifError('');setGifOpen(false);
  }
  return <div className="rich-composer-wrap">
    {(emojiOpen||gifOpen)&&<div className="composer-popover">
      {emojiOpen&&<div className="emoji-picker"><div className="popover-heading">Choose an emoji</div>{EMOJI_GROUPS.map(group=><div className="emoji-group" key={group.label}><small>{group.label}</small><div>{group.items.map(emoji=><button type="button" key={emoji} aria-label={'Insert '+emoji} onClick={()=>insertEmoji(emoji)}>{emoji}</button>)}</div></div>)}</div>}
      {gifOpen&&<div className="gif-picker"><div className="popover-heading">Share a GIF</div><p>Search Tenor, open a GIF, then right-click it and copy its image address.</p><form onSubmit={event=>{event.preventDefault();window.open('https://tenor.com/search/'+encodeURIComponent(gifQuery.trim()||'funny')+'-gifs','_blank','noopener,noreferrer')}}><input value={gifQuery} onChange={event=>setGifQuery(event.target.value)} placeholder="Search GIFs"/><button type="submit">Search Tenor</button></form><input value={gifUrl} onChange={event=>{setGifUrl(event.target.value);setGifError('')}} placeholder="Paste direct GIF URL"/>{gifError&&<small className="social-error">{gifError}</small>}<button type="button" className="insert-gif" onClick={insertGif}>Insert GIF</button></div>}
    </div>}
    <form className="composer rich-composer" onSubmit={onSubmit}>
      <button type="button" className="composer-tool" aria-label="Open emoji picker" aria-expanded={emojiOpen} onClick={()=>{setEmojiOpen(v=>!v);setGifOpen(false)}}>☺</button>
      <button type="button" className="composer-tool gif-tool" aria-label="Share a GIF" aria-expanded={gifOpen} onClick={()=>{setGifOpen(v=>!v);setEmojiOpen(false)}}>GIF</button>
      <textarea rows={1} value={value} onChange={event=>setValue(event.target.value)} onKeyDown={event=>{if(event.key==='Enter'&&!event.shiftKey&&enterToSend){event.preventDefault();event.currentTarget.form?.requestSubmit();}else if(event.key==='Enter'&&(event.ctrlKey||event.metaKey)&&!enterToSend){event.preventDefault();event.currentTarget.form?.requestSubmit();}}} maxLength={4000} placeholder={placeholder}/>
      <button className="send" type="submit" aria-label="Send message"><Send size={18}/></button>
    </form>
  </div>;
}

function FriendsPanel({account,friends,requests,username,setUsername,error,addFriend,answer,openDM}:{account:Account;friends:Friend[];requests:FriendRequest[];username:string;setUsername:(v:string)=>void;error:string;addFriend:(e:FormEvent<HTMLFormElement>)=>void;answer:(id:string,action:'accept'|'decline')=>void;openDM:(friend:Friend)=>void}) {
  const incoming=requests.filter(request=>request.recipient_id===account.id);
  const outgoing=requests.filter(request=>request.sender_id===account.id);
  return <section className="social-page"><h1>Friends</h1><p className="muted">Add people by username and manage your requests.</p>
    <form className="friend-add" onSubmit={addFriend}><input value={username} onChange={event=>setUsername(event.target.value)} placeholder="Enter a username" minLength={3} maxLength={24} required/><button className="auth-submit">Send request</button></form>
    {error&&<p className="social-error" role="alert">{error}</p>}
    <h2>Incoming requests <span>{incoming.length}</span></h2>
    {incoming.length===0?<p className="muted">No incoming requests.</p>:incoming.map(request=><div className="social-row" key={request.id}><b>{request.sender_username}</b><div><button onClick={()=>answer(request.id,'accept')}>Accept</button><button className="secondary" onClick={()=>answer(request.id,'decline')}>Decline</button></div></div>)}
    <h2>Sent requests <span>{outgoing.length}</span></h2>
    {outgoing.length===0?<p className="muted">No pending sent requests.</p>:outgoing.map(request=><div className="social-row" key={request.id}><b>{request.recipient_username}</b><small>Pending</small></div>)}
    <h2>All friends <span>{friends.length}</span></h2>
    {friends.length===0?<p className="muted">Your accepted friends will appear here.</p>:friends.map(friend=><div className="social-row" key={friend.id}><b>{friend.display_name||friend.username}</b><button onClick={()=>openDM(friend)}>Message</button></div>)}
  </section>;
}
function DirectMessagePanel({account,friend,messages,error,send,enterToSend=true,back}:{account:Account;friend:Friend;messages:DirectMessage[];error:string;send:(e:FormEvent<HTMLFormElement>,content:string)=>void;enterToSend?:boolean;back:()=>void}) {
  const [draft,setDraft]=useState('');
  return <section className="dm-page"><header><button className="secondary" onClick={back}>← Friends</button><h1>{friend.display_name||friend.username}</h1><small>Private conversation · live updates</small></header>
    <div className="dm-history">{messages.length===0?<p className="muted">No messages yet. Start the conversation.</p>:messages.map(message=><article className={'dm-message '+(message.sender_id===account.id?'mine':'')} key={message.id}><b>{message.sender_id===account.id?'You':friend.display_name||friend.username}</b><MessageBody content={message.content}/><time>{new Date(message.created_at).toLocaleString()}</time></article>)}</div>
    {error&&<p className="social-error">{error}</p>}
    <RichComposer value={draft} setValue={setDraft} onSubmit={event=>{void send(event,draft);setDraft('')}} placeholder={'Message '+(friend.display_name||friend.username)} enterToSend={enterToSend}/>
  </section>;
}

function AuthScreen({mode,setMode,submit,busy,error}:{mode:'login'|'register';setMode:(v:'login'|'register')=>void;submit:(e:FormEvent<HTMLFormElement>)=>void;busy:boolean;error:string}) {
  const [showPassword,setShowPassword]=useState(false);
  return <main className="auth-screen"><form className="auth-card" onSubmit={event=>{const data=new FormData(event.currentTarget);if(mode==='register'&&data.get('password')!==data.get('confirmPassword')){event.preventDefault();const node=event.currentTarget.querySelector<HTMLElement>('[data-password-error]');if(node)node.textContent='Passwords do not match.';return;}const node=event.currentTarget.querySelector<HTMLElement>('[data-password-error]');if(node)node.textContent='';submit(event);}}><div className="auth-logo">C</div><h1>{mode==='register'?'Create your account':'Welcome back'}</h1><p>{mode==='register'?'Join Chatty and start building your communities.':'Sign in to continue to Chatty.'}</p>
    {mode==='register'?<><label>Email<input name="email" type="email" autoComplete="email" maxLength={254} required/></label><label>Username<input name="username" autoComplete="username" minLength={3} maxLength={24} pattern="[A-Za-z0-9_]+" required/></label></>:<label>Email or username<input name="login" autoComplete="username" required/></label>}
    <label>Password<input name="password" type={showPassword?'text':'password'} autoComplete={mode==='register'?'new-password':'current-password'} minLength={mode==='register'?10:1} maxLength={128} required/></label>
    {mode==='register'&&<label>Confirm password<input name="confirmPassword" type={showPassword?'text':'password'} autoComplete="new-password" minLength={10} maxLength={128} required/></label>}
    <button type="button" className="password-visibility" onClick={()=>setShowPassword(value=>!value)}>{showPassword?'Hide passwords':'Show passwords'}</button>
    <small data-password-error className="auth-error" role="alert"></small>
    {error&&<div className="auth-error" role="alert">{error}</div>}<button className="auth-submit" disabled={busy}>{busy?'Please wait…':mode==='register'?'Create account':'Log in'}</button>
    <div className="auth-switch">{mode==='register'?'Already have an account?':'New to Chatty?'} <button type="button" onClick={()=>setMode(mode==='register'?'login':'register')}>{mode==='register'?'Log in':'Create an account'}</button></div>
    <small className="auth-footnote">Use a unique password. Account creation currently requires the Chatty API and PostgreSQL to be running.</small>
  </form></main>;
}
function Setting({title,desc}:{title:string;desc:string}){return <div className="setting-row"><div><b>{title}</b><p>{desc}</p></div></div>}
function Toggle({checked,change}:{checked:boolean;change:(v:boolean)=>void}){return <button className={'toggle '+(checked?'on':'')} role="switch" aria-checked={checked} onClick={()=>change(!checked)}><span/></button>}
