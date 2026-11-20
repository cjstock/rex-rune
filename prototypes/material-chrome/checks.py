"""PROTOTYPE: theme.rs's legibility test, run over generated palettes (light overlays use ink)."""
import json, colorsys
d=json.load(open('flat.json'))
M={'A':dict(ground='surface',popover='surface_container_high',accent='primary',warning='color3',danger='error'),
   'B':dict(ground='surface_container',popover='surface_container_highest',accent='primary',warning='color3',danger='color1'),
   'C':dict(ground='surface_container_lowest',popover='surface_container',accent='primary',warning='color3',danger='error')}
h2=lambda h:int(h[1:],16)
def mix(a,b,t): return sum(round(((a>>s)&255)*t+((b>>s)&255)*(1-t))<<s for s in (16,8,0))
def lum(c):
    ch=[((c>>s)&255)/255 for s in (16,8,0)]; ch=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in ch]
    return .2126*ch[0]+.7152*ch[1]+.0722*ch[2]
def cr(a,b): x,y=lum(a),lum(b); return (max(x,y)+.05)/(min(x,y)+.05)
def hue(c): return colorsys.rgb_to_hls(*[((c>>s)&255)/255 for s in (16,8,0)])[0]*360
def gap(a,b): g=abs(hue(a)-hue(b)); return min(g,360-g)
fails={}
for key,c in d.items():
    light=key.endswith('light')
    for m,r in M.items():
        p={k:h2(c[v]) for k,v in r.items()}; p['ink']=h2(c['on_surface']); p['muted']=h2(c['on_surface_variant'])
        over=p['ink'] if light else 0xffffff; k=0.9 if light else 1
        out=[]
        def need(name,v,t):
            if v<t: out.append(f"{name} {v:.2f}<{t}")
        need('muted/card',cr(p['muted'],p['ground']),4.5); need('ink/popover',cr(p['ink'],p['popover']),4.5)
        keys=mix(p['ink'],p['muted'],.5)
        for s,u,l in (('popover',p['popover'],0),('sidebar',p['ground'],.08)):
            need(f'keycaps/{s}',cr(keys,mix(over,u,(0.07+l)*k)),4.5)
        for kind,tone in (('sel',p['accent']),('dsel',p['danger'])):
            fill=mix(tone,p['popover'],.18); need(f'ink/{kind}',cr(p['ink'],fill),4.5)
            need(f'keycap/{kind}',cr(p['ink'],mix(over,fill,.12*k)),4.5); need(f'{kind} outline',cr(tone,p['popover']),3)
        for w in ('warning','danger'):
            for s in ('ground','popover'): need(f'{w}/{s}',cr(p[w],p[s]),3)
        need('warn~danger hue',gap(p['warning'],p['danger']),30)
        need('accent~danger hue',gap(p['accent'],p['danger']),30)
        fails[f'{key}.{m}']=out
for k,v in fails.items(): print(f"{k:52} {'ok' if not v else '; '.join(v)}")

ROLES={'A':dict(ground='surface',popover='surface_container_high',ink='on_surface',muted='on_surface_variant',accent='primary',accent2='tertiary',live='secondary',warning='color3',danger='error',on_accent='on_primary',terminal_selection='primary_container'),
       'B':dict(ground='surface_container',popover='surface_container_highest',ink='on_surface',muted='on_surface_variant',accent='primary',accent2='secondary',live='color2',warning='color3',danger='color1',on_accent='on_primary',terminal_selection='secondary_container'),
       'C':dict(ground='surface_container_lowest',popover='surface_container',ink='on_surface',muted='on_surface_variant',accent='primary',accent2='tertiary',live='color2',warning='color3',danger='error',on_accent='on_primary',terminal_selection='surface_variant')}
out={'roles':ROLES,'palettes':{}}
for key,c in d.items():
    for m,r in ROLES.items():
        out['palettes'][f'{key}.{m}']={'tokens':{t:c[role] for t,role in r.items()},'fails':fails[f'{key}.{m}']}
json.dump(out,open('viewer/palettes.json','w'))
