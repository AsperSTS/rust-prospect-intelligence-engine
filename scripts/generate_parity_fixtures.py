#!/usr/bin/env python3
"""Development-only: freeze synthetic Python outputs for Rust regression tests.
The installed application never invokes Python or reads this script.
"""
import argparse,datetime,json,sys
from pathlib import Path

parser=argparse.ArgumentParser()
parser.add_argument('--source',type=Path,required=True)
args=parser.parse_args()
sys.path.insert(0,str(args.source.resolve()))
from prospectos import acciones,config,contacto,giros,grupos,mensajes,scoring,senales,texto
config.OWNER_NAME='Demo Validación'
config.OWNER_WHATSAPP='2221234567'
config.OWNER_CREDENTIAL='Participé en proyectos de automatización.'
config.TEMPLATES_PATH=args.source/'config/templates.toml'
date=datetime.date(2026,10,11)
cases=[]
codes=['336310','336100','332310','332710','332999','331110','333111','311111','431110','484121','488111','492110','493110','236111','811111','999999']
states=['nuevo','revisado','contactado','respondio','respuesta_positiva','reunion','propuesta','cliente','no_interesado','sin_respuesta','descartado']
for code in codes:
 for state in states:
  for channel in range(4):
   i=len(cases)
   row=dict(id=str(100000+i),nombre=['MAQUINAS DIESEL SUC. PUEBLA S.A. DE C.V.','ACME B2B S.A.P.I. DE C.V.','Acero & Diseño','TALLER'][i%4],razon_social='',clase_id=code,clase_actividad='Fabricación de productos industriales',estrato=['11 a 30 personas','31 a 50 personas','51 a 100 personas','0 a 5 personas',''][i%5],tipo_establecimiento=['fijo','','semifijo'][i%3],corredor_nombre=['Parque Industria','Sin nombre','Mercado Alfa',''][i%4],corredor_tipo=['parque industrial','zona industrial','mercado',''][i%4],estado=state,telefono='2221234567' if channel in (1,3) else '',correo='ventas@acme.mx' if channel in (2,3) else '',correo_web='',whatsapp='',sitio='https://acme.mx' if channel==3 else '',group_key='id:'+str(100000+i),possible_chain=0,possible_chain_count=1,group_reason='',creado_en=['2026-10-01','2026-01-01',''][i%3],website_checked_at=['2026-10-05','2025-01-01',''][i%3],proximo_seguimiento='2026-10-09' if i%2 else '2026-10-20',toques=i%3+1)
   types=[[],['cotizacion'],['inventario','logistica','iso'],['fabricacion','proveedores'],['sobre_pedido','mantenimiento']][i%5]
   signals=[dict(signal_type=t,signal_text='Evidencia pública de '+t,signal_source_url='https://acme.mx/servicios') for t in types]
   g=giros.clasificar(code,row['clase_actividad']);row.update(giro=g.giro,giro_display=g.display)
   fit,fit_items=scoring.fit(row,g,types);priority,prio_items,_=scoring.priority(row,fit,types,False,date)
   hypothesis,basis,confidence=giros.hipotesis(g,[senales.etiqueta(t) for t in senales.relacionadas(types)[:3]])
   analysis=dict(giro=g.giro,giro_display=g.display,fit_score=fit,fit_breakdown=fit_items,priority_score=priority,priority_breakdown=prio_items,hypothesis_text=hypothesis,hypothesis_basis=basis,hypothesis_confidence=confidence,action=acciones.recommend_next_action(row,fit,bool(types),date),group={k:row[k] for k in ['group_key','possible_chain','possible_chain_count','group_reason']})
   def message(m):return dict(canal=m.canal,variante=m.variante,cuerpo=m.cuerpo,asunto=m.asunto,problemas=m.problemas)
   cases.append(dict(row=row,signals=signals,expected=analysis,whatsapp=message(mensajes.primer_contacto(row,'whatsapp',signals)),email=message(mensajes.primer_contacto(row,'email',signals)),followups=[message(mensajes.seguimiento(row,p)) for p in (1,2,3)]))
group_rows=[dict(id=str(i),nombre=n,razon_social=rs,sitio=web,telefono=tel,entidad='Puebla') for i,(n,rs,web,tel) in enumerate([('ACME','Acme S.A. de C.V.','https://acme.mx','2221234567'),('ACME Planta','ACME SA DE CV','',''),('Logística Beta','','https://beta.mx','2221234567'),('ACME Comercio','','https://beta.mx',''),('Taller Alfa','','https://facebook.com',''),('Taller Alfa','','',''),('Taller Alfa','','',''),('Nombre similar','','',''),('Nombre similares','','','')])]
texts=['Somos fabricantes sobre pedido. Ofrecemos control de calidad y mantenimiento para las empresas de Puebla.','© Empresa™ \ue001 Toggle Menu Almacén y logística. Hacemos inventario y entregas a tiempo.','Cotiza con nosotros. Servicio técnico y soporte para equipos industriales.','Menu Productos Servicios Contacto ISO 9001 Tenemos certificación ISO 9001 y un sistema de gestión de calidad.','Nuestros proyectos y proveedores atienden servicios en varias sucursales.']
fixtures=dict(rounding=[round(i*0.35) for i in range(101)],date=str(date),settings=dict(owner_name=config.OWNER_NAME,owner_whatsapp=config.OWNER_WHATSAPP,owner_credential=config.OWNER_CREDENTIAL),cases=cases,group_rows=group_rows,groups=grupos.agrupar(group_rows),texts=[dict(input=t,clean=texto.limpiar(t),signals=senales.detectar(t,'https://acme.mx')) for t in texts],names=[dict(raw=n,clean=mensajes.limpiar_nombre(n)) for n in ['MAQUINAS DIESEL SUC. PUEBLA S.A. DE C.V.','ACME B2B S.A.P.I. DE C.V.','SERVICIOS DE SAN PEDRO S.A. DE C.V.','ABC CNC TEC MEX','Gas del Sur','']])
out=Path(__file__).resolve().parents[1]/'crates/core/tests/fixtures/parity.json'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(json.dumps(fixtures,ensure_ascii=False,separators=(',',':')))
print(f'Fixtures sintéticos: {len(cases)} escenarios, {len(group_rows)} agrupaciones, {len(texts)} textos; sin contactos reales ni tokens.')
