import { useState } from 'react';
import { Bell, Hash, Headphones, MessageCircle, Plus, Search, Settings, Shield, Users, Volume2, ChevronDown, Send, Download, X } from 'lucide-react';

type Message = { name: string; text: string; time: string };
const seed: Message[] = [
  { name: 'Nova', time: '10:42', text: 'Welcome to the Chatty preview! This is the start of a new community.' },
  { name: 'PixelPilot', time: '10:44', text: 'The interface is taking shape. More features will arrive as the backend grows.' },
];
export default function App() {
  const [page,setPage]=useState<'chat'|'settings'>('chat');
  const [channel,setChannel]=useState('general');
  const [messages,setMessages]=useState(seed);
  const [draft,setDraft]=useState('');
  const [search,setSearch]=useState('');
  const [dark,setDark]=useState(true);
  const [compact,setCompact]=useState(false);
  const [notifications,setNotifications]=useState(true);
  const [updateOpen,setUpdateOpen]=useState(false);
  const [tab,setTab]=useState('My Account');
  function send(e:React.FormEvent){e.preventDefault();const text=draft.trim();if(!text)return;setMessages(v=>[...v,{name:'You',time:new Date().toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'}),text}]);setDraft('');}
  const shown=messages.filter(m=>(m.text+m.name).toLowerCase().includes(search.toLowerCase()));
  return <div className={dark?'app dark':'app light'}>
    <aside className="servers"><button className="server-logo active">C</button><div className="server-divider"/><button className="server-logo"><Users/></button><button className="server-logo add"><Plus/></button></aside>
    <aside className="sidebar">
      <button className="workspace"><span className="brand-mark">C</span><span><b>Chatty</b><small>Community space</small></span><ChevronDown size={17}/></button>
      <button className="nav-item"><MessageCircle size={18}/> Direct Messages</button>
      <div className="section-label">TEXT CHANNELS <Plus size={15}/></div>
      {['general','announcements','media','off-topic'].map(c=><button key={c} className={'channel '+(channel===c?'selected':'')} onClick={()=>{setChannel(c);setPage('chat')}}><Hash size={17}/>{c}</button>)}
      <div className="section-label voice-label">VOICE CHANNELS <Plus size={15}/></div>
      <button className="channel"><Volume2 size={17}/> Lounge</button><button className="channel"><Headphones size={17}/> Gaming</button>
      <div className="sidebar-bottom"><div className="user-avatar">O</div><span className="user-meta"><b>OGBONEX</b><small>Local preview</small></span><button className="icon-button" title="Settings" onClick={()=>setPage('settings')}><Settings size={18}/></button></div>
    </aside>
    <main className="main">
      <header className="topbar"><div className="channel-title"><Hash size={20}/><b>{page==='settings'?'User Settings':channel}</b></div><div className="top-actions"><label className="search"><Search size={16}/><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search local messages"/></label><button className="icon-button" title="Toggle notifications" onClick={()=>setNotifications(v=>!v)}><Bell size={19}/></button><button className="update-button" onClick={()=>setUpdateOpen(v=>!v)}><Download size={17}/> Updates</button><button className="icon-button" title="Settings" onClick={()=>setPage(page==='settings'?'chat':'settings')}><Settings size={19}/></button></div></header>
      {updateOpen&&<div className="update-popover"><button className="close" onClick={()=>setUpdateOpen(false)}><X size={16}/></button><b>Preview build</b><p>Chatty v0.1.0</p><small>Signed update delivery will be connected for desktop releases in a later milestone.</small></div>}
      {page==='settings'?<section className="settings-layout"><nav className="settings-nav"><h2>Settings</h2>{['My Account','Privacy & Safety','Appearance','Notifications','Voice & Video','Text & Messages','Keybinds','Activity','Language & Region','Accessibility','Storage','Advanced','Updates'].map(t=><button key={t} className={tab===t?'setting-active':''} onClick={()=>setTab(t)}>{t}</button>)}</nav><div className="settings-content"><h1>{tab}</h1><p className="muted">Manage your Chatty experience.</p>{tab==='Appearance'?<><Setting title="Dark theme" desc="Use the dark interface."/><Toggle checked={dark} change={setDark}/><Setting title="Compact messages" desc="Reduce spacing between messages."/><Toggle checked={compact} change={setCompact}/></>:tab==='Notifications'?<><Setting title="In-app notifications" desc="Show notification indicators in Chatty."/><Toggle checked={notifications} change={setNotifications}/></>:tab==='Updates'?<Setting title="Application updates" desc="Signed update checks will be connected to platform releases."/>:<><Setting title="Settings section" desc="This page is reserved for the account and device preferences milestone."/><div className="notice"><Shield size={18}/> Security and privacy settings will be enforced by the API, not just the interface.</div></>}</div></section>:<section className="chat-area"><div className="welcome"><div className="welcome-icon"><Hash/></div><h1>Welcome to #{channel}</h1><p>This is the beginning of the #{channel} channel.</p></div><div className="messages">{shown.map((m,i)=><article className={'message '+(compact?'compact':'')} key={i}><div className="avatar">{m.name[0]}</div><div><div className="message-meta"><b>{m.name}</b><time>{m.time}</time></div><p>{m.text}</p></div></article>)}</div><form className="composer" onSubmit={send}><input value={draft} onChange={e=>setDraft(e.target.value)} placeholder={'Message #'+channel}/><button className="send" type="submit"><Send size={18}/></button></form></section>}
      <footer className="statusbar"><span><i/> Local preview · API connection not configured</span><span>Chatty · 0.1.0</span></footer>
    </main>
  </div>;
}
function Setting({title,desc}:{title:string;desc:string}){return <div className="setting-row"><div><b>{title}</b><p>{desc}</p></div></div>}
function Toggle({checked,change}:{checked:boolean;change:(v:boolean)=>void}){return <button className={'toggle '+(checked?'on':'')} role="switch" aria-checked={checked} onClick={()=>change(!checked)}><span/></button>}
