# Text swimlane for the 2026-09-23 lightning round (yaks-77a5).
# One column = 10s, from 17:35:50. Spawn ~17:36:05 for all agents
# (lane commit - duration). Legend: = agent running, C lane commit,
# . waiting for merge, M squash-merged to main, ? design ask filed.
def s(t):
    h, m, sec = map(int, t.split(':')); return (h*3600+m*60+sec) - (17*3600+35*60+50)
SPAWN = s('17:36:05'); COL = 10; W = s('17:47:40')//COL + 1
lanes = [  # name, done(=agent end or lane commit), merged
 ('scroll',   '17:37:35', '17:38:18'),
 ('openid',   '17:39:46', '17:41:06'),
 ('width',    '17:40:38', '17:42:01'),
 ('navpos',   '17:41:43', '17:43:01'),
 ('form',     '17:43:03', '17:43:28'),
 ('polish',   '17:43:12', '17:43:45'),
 ('slaughter','17:43:20', '17:44:03'),
 ('mouse',    '17:43:34', '17:44:29'),
 ('paste',    '17:44:00', '17:45:18'),
 ('attach',   '17:44:56', '17:45:49'),
 ('labels',   '17:45:18', '17:46:23'),
]
scouts = [('scout:683f','17:39:11'),('scout:158d','17:39:42'),('scout:953f','17:40:08')]
def row(done, merged=None, end_ch='C'):
    r = [' ']*W; a, d = SPAWN//COL, s(done)//COL
    for i in range(a, d): r[i] = '='
    r[d] = end_ch
    if merged:
        m = s(merged)//COL
        for i in range(d+1, m): r[i] = '.'
        r[m] = 'M'
    return ''.join(r)
hdr = [' ']*W
for i in range(W):
    t = i*COL + 50
    if t % 60 == 0:
        lab = f":{35 + t//60:02d}"
        for j,c in enumerate(lab):
            if i+j < W: hdr[i+j] = c
print(f"{'17h':<11}" + ''.join(hdr))
claim = [' ']*W; claim[s('17:35:54')//COL] = 'K'
print(f"{'claim':<11}" + ''.join(claim))
for n,d,m in lanes: print(f"{n:<11}" + row(d, m))
for n,d in scouts: print(f"{n:<11}" + row(d, None, '?'))
co = [' ']*W
for t in ['17:38:18','17:41:06','17:42:01','17:43:01','17:43:28','17:43:45','17:44:03','17:44:29','17:45:18','17:45:49','17:46:23','17:47:05']:
    co[s(t)//COL] = 'M'
co[s('17:38:18')//COL] = 'M'; co[s('17:40:00')//COL] = 'b'
print(f"{'main':<11}" + ''.join(co))
print("\nK claim commit b196a1e   = agent running   C lane commit   . awaiting merge")
print("M squash-merge on main    ? scout ask filed   b shared-target warning broadcast (~17:40)")
print("1 column = 10s. All 14 agents spawned in one message at ~17:36:05.")
