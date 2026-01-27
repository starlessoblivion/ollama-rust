// script.js (full) - updated submenu hide/show behavior
let currentRunner = 'ollama';
let currentModel = null;

// helpers for viewport/pointer
(function () {
    function updateVH() { document.documentElement.style.setProperty('--vh', window.innerHeight + 'px'); }
    function setPointerClass() {
        try {
            const isCoarse = window.matchMedia && window.matchMedia('(pointer: coarse)').matches;
            const smallScreen = window.innerWidth <= 720;
            if (isCoarse || smallScreen) {
                document.body.classList.add('touch');
                document.body.classList.remove('desktop');
            } else {
                document.body.classList.add('desktop');
                document.body.classList.remove('touch');
            }
        } catch (e) {
            document.body.classList.add('desktop');
        }
    }
    updateVH();
    setPointerClass();
    let resizeTimeout;
    window.addEventListener('resize', () => {
        clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => { updateVH(); setPointerClass(); }, 150);
    });
    window.addEventListener('orientationchange', () => { setTimeout(updateVH, 300); setPointerClass(); });
}());

// --- status/models population ---
function refreshStatusAndModels() {
    fetch('/status')
    .then(r => r.json())
    .then(data => {
        const checkbox = document.getElementById('ollama-toggle');
        const submenu = document.getElementById('ollama-models');
        if (checkbox) {
            checkbox.checked = !!data.running;
            checkbox.setAttribute('aria-checked', checkbox.checked ? 'true' : 'false');
            checkbox.disabled = false;
        }
        if (!submenu) return;
        submenu.innerHTML = '';
        if (data.running && Array.isArray(data.models)) {
            data.models.forEach(m => {
                const el = document.createElement('div');
                el.className = 'model-option';
                el.textContent = m;
                el.tabIndex = 0;
                el.onclick = () => { selectModel('ollama', m); hideModelMenu(); };
                el.addEventListener('keydown', (e) => { if (e.key === 'Enter') { selectModel('ollama', m); hideModelMenu(); }});
                el.addEventListener('touchend', () => { selectModel('ollama', m); hideModelMenu(); }, { passive: true });
                submenu.appendChild(el);
            });
            // auto-select first model if none selected
            if (data.models.length > 0 && !currentModel) {
                selectModel('ollama', data.models[0]);
            }
        } else {
            setTimeout(refreshStatusAndModels, 250);
        }
    })
    .catch((err) => {
        console.warn('refreshStatusAndModels error', err);
        const checkbox = document.getElementById('ollama-toggle');
        if (checkbox) {
            checkbox.checked = false;
            checkbox.setAttribute('aria-checked', 'false');
            checkbox.disabled = false;
        }
    });
}

// menu open/close (top-level menu)
function toggleModelMenu(e) {
    if (e && e.stopPropagation) { e.stopPropagation(); e.preventDefault && e.preventDefault(); }
    const menu = document.getElementById('model-menu');
    if (!menu) return;
    const opening = menu.classList.contains('hidden');
    if (opening) {
        menu.classList.remove('hidden');
        menu.setAttribute('aria-hidden', 'false');
        document.getElementById('model-button')?.setAttribute('aria-expanded', 'true');
    } else {
        hideModelMenu();
    }
}

function hideModelMenu() {
    const menu = document.getElementById('model-menu');
    if (!menu) return;
    menu.classList.add('hidden');
    menu.setAttribute('aria-hidden', 'true');
    document.getElementById('model-button')?.setAttribute('aria-expanded', 'false');

    // also hide any open runner subpanels
    document.querySelectorAll('.runner-item[aria-expanded="true"]').forEach(r => {
        const panel = r.querySelector('.models-panel');
        if (panel) panel.classList.add('hidden'), panel.setAttribute('aria-hidden', 'true');
        r.setAttribute('aria-expanded', 'false');
    });
}

// runner-item hover/tap behavior (existing)
function wireRunnerItemBehavior() {
    const runner = document.getElementById('runner-ollama');
    if (!runner) return;
    const panel = runner.querySelector('.models-panel') || document.getElementById('models-panel');

    // mouse hover shows submenu (kept for compatibility)
    runner.addEventListener('mouseenter', () => {
        if (panel) {
            panel.classList.remove('hidden');
            panel.setAttribute('aria-hidden', 'false');
            runner.setAttribute('aria-expanded', 'true');
        }
    });
    runner.addEventListener('mouseleave', () => {
        if (panel) {
            // do nothing here — improved handler below manages hide with delay
        }
    });

    // for keyboard accessibility: focus shows, blur hides
    runner.addEventListener('focus', () => {
        if (panel) {
            panel.classList.remove('hidden');
            panel.setAttribute('aria-hidden', 'false');
            runner.setAttribute('aria-expanded', 'true');
        }
    }, true);
    runner.addEventListener('blur', () => {
        if (panel) {
            panel.classList.add('hidden');
            panel.setAttribute('aria-hidden', 'true');
            runner.setAttribute('aria-expanded', 'false');
        }
    }, true);

    // touch/click toggles submenu (single-tap to open, second tap to select or close)
    let touchOpen = false;
    runner.addEventListener('click', (e) => {
        // prevent the top-level menu from closing when interacting with runner
        e.stopPropagation();
        if (!panel) return;
        const isHidden = panel.classList.contains('hidden');
        if (isHidden) {
            panel.classList.remove('hidden');
            panel.setAttribute('aria-hidden', 'false');
            runner.setAttribute('aria-expanded', 'true');
            touchOpen = true;
        } else {
            panel.classList.add('hidden');
            panel.setAttribute('aria-hidden', 'true');
            runner.setAttribute('aria-expanded', 'false');
            touchOpen = false;
        }
    }, { passive: true });
}

// Improved runner submenu wiring so moving pointer into the panel keeps it open
function wireRunnerSubmenu() {
    const runner = document.getElementById('runner-ollama');
    if (!runner) return;
    const panel = runner.querySelector('.models-panel') || document.getElementById('models-panel');
    if (!panel) return;

    let hideTimer = null;
    const HIDE_DELAY = 150; // ms

    function clearHideTimer() {
        if (hideTimer) { clearTimeout(hideTimer); hideTimer = null; }
    }

    function scheduleHide() {
        clearHideTimer();
        hideTimer = setTimeout(() => {
            panel.classList.add('hidden');
            panel.setAttribute('aria-hidden', 'true');
            runner.setAttribute('aria-expanded', 'false');
            hideTimer = null;
        }, HIDE_DELAY);
    }

    function showPanel() {
        clearHideTimer();
        panel.classList.remove('hidden');
        panel.setAttribute('aria-hidden', 'false');
        runner.setAttribute('aria-expanded', 'true');
    }

    // Runner area: when pointer enters runner, show panel
    runner.addEventListener('mouseenter', showPanel);
    // When pointer leaves runner, schedule hide (but it may be cancelled by entering panel)
    runner.addEventListener('mouseleave', scheduleHide);

    // Panel area: entering panel cancels hide and keeps it shown
    panel.addEventListener('mouseenter', showPanel);
    // leaving panel schedules hide
    panel.addEventListener('mouseleave', scheduleHide);

    // keyboard: keep same behavior — focusin shows, focusout hides if focus leaves both runner and panel
    runner.addEventListener('focusin', showPanel);
    runner.addEventListener('focusout', (e) => {
        const related = e.relatedTarget;
        if (!runner.contains(related) && !panel.contains(related)) {
            scheduleHide();
        }
    });

    // panel focus handling: if focus moves outside both, hide
    panel.addEventListener('focusout', (e) => {
        const related = e.relatedTarget;
        if (!runner.contains(related) && !panel.contains(related)) {
            scheduleHide();
        }
    });

    // touch / click: toggle on click, stop propagation so top-level menu doesn't immediately close
    panel.addEventListener('click', (e) => { e.stopPropagation(); }, { passive: true });
    runner.addEventListener('click', (e) => {
        e.stopPropagation();
        if (panel.classList.contains('hidden')) showPanel();
        else scheduleHide();
    }, { passive: true });

        // If user clicks outside the top-level menu, ensure panel hides immediately
        window.addEventListener('click', (e) => {
            const md = document.getElementById('model-menu');
            if (!md) return;
            if (!md.contains(e.target)) {
                clearHideTimer();
                panel.classList.add('hidden');
                panel.setAttribute('aria-hidden', 'true');
                runner.setAttribute('aria-expanded', 'false');
            }
        });
}

// model selection
function selectModel(runner, model) {
    currentRunner = runner;
    currentModel = model;
    const btn = document.getElementById('model-button');
    if (btn) btn.textContent = `🧠 ${runner}: ${model}`;
    document.querySelectorAll('.model-option').forEach(el => el.classList.remove('selected'));
    const selected = [...document.querySelectorAll('.model-option')].find(el => el.textContent === model);
    if (selected) selected.classList.add('selected');
    hideModelMenu();
}

// toggle ollama serve
function toggleOllama() {
    const checkbox = document.getElementById('ollama-toggle');
    if (!checkbox) return;

    // optimistic UI: disable until server confirms new state
    checkbox.disabled = true;

    fetch('/toggle-ollama', { method: 'POST' })
    .then(r => r.json())
    .then(data => {
        // ensure checkbox matches backend
        checkbox.checked = !!data.running;
        checkbox.setAttribute('aria-checked', checkbox.checked ? 'true' : 'false');
        checkbox.disabled = false;

        // refresh models if started
        refreshStatusAndModels();
    })
    .catch(err => {
        console.warn('Toggle error:', err);
        // restore visual state by re-syncing with server
        refreshStatusAndModels();
    });
}

// thinking placeholder builder (dots + brain)
function thinkingPlaceholder() {
    const wrapper = document.createElement('span');
    wrapper.className = 'thinking';

    const brain = document.createElement('span');
    brain.className = 'brain';
    brain.textContent = '🧠';

    const dotsWrap = document.createElement('span');
    dotsWrap.className = 'thinking-dots';
    for (let i = 0; i < 3; i++) {
        const d = document.createElement('span');
        d.className = 'thinking-dot';
        dotsWrap.appendChild(d);
    }

    wrapper.appendChild(brain);
    wrapper.appendChild(dotsWrap);

    return { wrapper, brainNode: brain, dotsNode: dotsWrap };
}

// send helpers: improved DOM management for AI text population and animations
function setSendButtonSending(isSending) {
    const btn = document.getElementById('send-button');
    if (!btn) return;
    if (isSending) { btn.classList.add('sending'); btn.disabled = true; }
    else { btn.classList.remove('sending'); btn.disabled = false; }
}

function autoResizeTextarea() {
    const ta = document.getElementById('prompt-input');
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = ta.scrollHeight + 'px';
}

function addBubble(text, className) {
    const bubble = document.createElement('div');
    bubble.className = `chat-bubble ${className}`;
    bubble.textContent = text;
    const chatWindow = document.getElementById('chat-window');
    chatWindow.appendChild(bubble);
    chatWindow.scrollTop = chatWindow.scrollHeight;
}

function sendPrompt() {
    const promptInput = document.getElementById('prompt-input');
    const prompt = promptInput ? promptInput.value : '';
    if (!prompt.trim() || !currentModel) return;

    addBubble(prompt, 'user-bubble');
    if (promptInput) { promptInput.value = ''; autoResizeTextarea(); }

    setSendButtonSending(true);

    // create AI bubble with placeholder (brain + dots)
    const aiBubble = document.createElement('div');
    aiBubble.className = 'chat-bubble ai-bubble';
    aiBubble.dataset.streamingStarted = 'false';

    const placeholder = thinkingPlaceholder();
    aiBubble.appendChild(placeholder.wrapper);

    // create a dedicated element to hold streaming text (hidden until first chunk)
    const aiText = document.createElement('div');
    aiText.className = 'ai-text';
    aiText.style.whiteSpace = 'pre-wrap';
    aiText.style.display = 'none'; // show once streaming starts
    aiText.setAttribute('aria-live', 'polite');
    aiBubble.appendChild(aiText);

    const chatWindow = document.getElementById('chat-window');
    chatWindow.appendChild(aiBubble);
    chatWindow.scrollTop = chatWindow.scrollHeight;

    // brain pulse timer (start after 10s)
    let brainPulseTimer = setTimeout(() => {
        placeholder.brainNode.classList.add('brain-blue');
    }, 10000);

    fetch('/stream-run', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: currentModel, runner: currentRunner, prompt })
    }).then(response => {
        if (!response.body) {
            return response.json().then(j => {
                clearTimeout(brainPulseTimer);
                placeholder.brainNode.classList.remove('brain-blue');
                if (aiBubble.contains(placeholder.wrapper)) aiBubble.removeChild(placeholder.wrapper);
                aiText.style.display = 'block';
                aiText.textContent = j.response || 'No response';
                setSendButtonSending(false);
            }).catch(() => {
                clearTimeout(brainPulseTimer);
                placeholder.brainNode.classList.remove('brain-blue');
                if (aiBubble.contains(placeholder.wrapper)) aiBubble.removeChild(placeholder.wrapper);
                aiText.style.display = 'block';
                aiText.textContent = '🧠 Error getting response.';
                setSendButtonSending(false);
            });
        }

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffered = '';
        let fullText = '';

        function read() {
            reader.read().then(({ done, value }) => {
                if (done) {
                    clearTimeout(brainPulseTimer);
                    placeholder.brainNode.classList.remove('brain-blue');
                    setSendButtonSending(false);
                    chatWindow.scrollTop = chatWindow.scrollHeight;
                    return;
                }
                buffered += decoder.decode(value, { stream: true });

                let parts = buffered.split(/\n\n/);
                buffered = parts.pop();

                for (const part of parts) {
                    const lines = part.split(/\n/);
                    for (const line of lines) {
                        if (line.startsWith('data:')) {
                            const payload = line.slice(5);
                            if (payload === '__END__') {
                                clearTimeout(brainPulseTimer);
                                placeholder.brainNode.classList.remove('brain-blue');
                                setSendButtonSending(false);
                                reader.cancel();
                                return;
                            } else {
                                // on first chunk: remove placeholder dots but keep brain (if desired) and show text container
                                if (aiBubble.dataset.streamingStarted === 'false') {
                                    aiBubble.dataset.streamingStarted = 'true';
                                    clearTimeout(brainPulseTimer);
                                    placeholder.brainNode.classList.remove('brain-blue');
                                    // remove the dots-only area but keep brain icon for continuity
                                    if (placeholder.wrapper && placeholder.wrapper.parentNode === aiBubble) {
                                        // replace wrapper with a smaller brain-only element if desired, else remove entirely
                                        aiBubble.removeChild(placeholder.wrapper);
                                    }
                                    aiText.style.display = 'block';
                                }
                                // append streaming payload to the aiText element
                                fullText += payload;
                                aiText.textContent = fullText;
                                chatWindow.scrollTop = chatWindow.scrollHeight;
                            }
                        }
                    }
                }
                read();
            }).catch(() => {
                clearTimeout(brainPulseTimer);
                placeholder.brainNode.classList.remove('brain-blue');
                setSendButtonSending(false);
                if (aiBubble.dataset.streamingStarted === 'false') {
                    if (aiBubble.contains(placeholder.wrapper)) aiBubble.removeChild(placeholder.wrapper);
                    aiText.style.display = 'block';
                    aiText.textContent = '🧠 Error streaming response.';
                    aiBubble.dataset.streamingStarted = 'true';
                } else {
                    aiText.textContent += '\n\n[stream error]';
                }
            });
        }
        read();
    }).catch(() => {
        clearTimeout(brainPulseTimer);
        placeholder.brainNode.classList.remove('brain-blue');
        if (aiBubble.contains(placeholder.wrapper)) aiBubble.removeChild(placeholder.wrapper);
        aiText.style.display = 'block';
        aiText.textContent = '🧠 Error connecting to stream.';
        setSendButtonSending(false);
    });
}

// textarea autosize wiring
const promptInput = document.getElementById('prompt-input');
if (promptInput) {
    promptInput.addEventListener('input', autoResizeTextarea);
    promptInput.addEventListener('keydown', function(e) {
        if (e.key === 'Enter' && !e.altKey && !e.shiftKey) { e.preventDefault(); sendPrompt(); }
        else if (e.key === 'Enter' && (e.altKey || e.shiftKey)) {
            const cursorPos = this.selectionStart;
            const value = this.value;
            this.value = value.substring(0, cursorPos) + "\n" + value.substring(cursorPos);
            this.selectionStart = this.selectionEnd = cursorPos + 1;
            e.preventDefault();
        }
    });
}

// initial wiring
window.addEventListener('DOMContentLoaded', () => {
    refreshStatusAndModels();

    // close dropdown on outside click
    window.addEventListener('click', (e) => {
        const md = document.querySelector('.model-dropdown');
        if (!md) return;
        if (!md.contains(e.target)) hideModelMenu();
    });

        // wire runner submenu behavior
        wireRunnerItemBehavior(); // existing behavior wiring (hover/click)
wireRunnerSubmenu(); // improved wiring to keep panel open when pointer moves into it

const sendBtn = document.getElementById('send-button');
if (sendBtn) {
    sendBtn.addEventListener('click', (e) => { if (!sendBtn.dataset.processing) { sendBtn.dataset.processing = '1'; sendPrompt(); setTimeout(() => delete sendBtn.dataset.processing, 300); }});
    sendBtn.addEventListener('touchend', (e) => { if (!sendBtn.dataset.touchHandled) { sendBtn.dataset.touchHandled = '1'; sendPrompt(); setTimeout(() => delete sendBtn.dataset.touchHandled, 300); } }, { passive: true });
}

// focus input for faster interaction
const ta = document.getElementById('prompt-input');
if (ta) ta.focus();
});
