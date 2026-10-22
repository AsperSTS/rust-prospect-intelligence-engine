"""Seed a 100% fictional database for portfolio screenshots.

Usage: python seed_demo.py <PROSPECTOS_HOME>
Run `prospectos --home <PROSPECTOS_HOME> inspect` first so the schema exists,
and `prospectos --home <PROSPECTOS_HOME> analizar` afterwards to compute scores.
All names, phones and domains are invented (.test domains, 555 phone prefix).
"""
import random, sqlite3, sys
from datetime import date, datetime, timedelta
from pathlib import Path

home = Path(sys.argv[1])
con = sqlite3.connect(home / "prospectos.db")
rnd = random.Random(11)
today = date.today()

GIROS = {
    "metal": ("332710", "Maquinado de piezas metálicas sobre diseño", "Maquinados", ["sobre_pedido", "cotizacion", "calidad"]),
    "estruct": ("332320", "Fabricación de estructuras metálicas", "Estructuras", ["sobre_pedido", "cotizacion", "proyectos"]),
    "auto": ("336390", "Fabricación de otras partes para vehículos automotores", "Autopartes", ["iso", "calidad", "produccion", "entregas"]),
    "transp": ("484210", "Autotransporte foráneo de carga general", "Transportes", ["flotilla", "entregas", "mantenimiento"]),
    "log": ("488519", "Otros servicios relacionados con el transporte", "Logística", ["logistica", "almacen", "inventario", "entregas"]),
    "alm": ("493110", "Servicios de almacenamiento general", "Almacenes", ["almacen", "inventario", "distribucion"]),
    "const": ("236221", "Edificación de naves y plantas industriales", "Construcciones", ["proyectos", "cotizacion", "proveedores"]),
    "taller": ("811121", "Reparación y mantenimiento de maquinaria y equipo", "Mantenimiento Industrial", ["servicio_tecnico", "mantenimiento", "cotizacion"]),
    "mayor": ("431110", "Comercio al por mayor de abarrotes", "Distribuidora", ["distribucion", "inventario", "proveedores", "sucursales"]),
}
TEXT = {
    "sobre_pedido": "Fabricación sobre pedido según especificaciones del cliente",
    "cotizacion": "Solicita tu cotización en línea", "calidad": "Control de calidad en cada lote",
    "proyectos": "Proyectos industriales llave en mano", "iso": "Sistema de gestión de calidad ISO 9001",
    "produccion": "Líneas de producción y manufactura", "entregas": "Entregas a nivel nacional",
    "flotilla": "Flotilla propia de unidades de carga", "mantenimiento": "Mantenimiento preventivo de unidades",
    "logistica": "Logística integral", "almacen": "Almacenamiento y control de almacén",
    "inventario": "Gestión de inventarios", "distribucion": "Distribución a mayoristas",
    "servicio_tecnico": "Servicio técnico especializado", "proveedores": "Red de proveedores certificados",
    "sucursales": "Sucursales en el centro del país",
}
LUGARES = ["Aldebarán", "Tezcatl", "Volcán Grande", "Tecalli", "Citlali", "Cumbres del Oriente", "Altiplano", "Malinche",
    "Tlaxcalli", "Cholultecas", "Atoyac", "Tepetlapa", "Calpan", "Axocopan", "Texoloc", "Xonaca", "Cuexcomate", "Quetzal",
    "Zacatlán Sur", "Nahuatl", "Tepeyac Norte", "Huehuetla", "Tenayuca", "Ometepec", "Zoquiapan", "Amalucan", "Coatepec",
    "Mixquiapan", "Teotihuacán", "Chalchihuites", "Xaltocan", "Tochtepec", "Apanco", "Ixtapa Verde", "Papalotla", "Tecuexcomac",
    "Sierra Alta", "Norte Industrial", "Delta", "Orión", "Meridiano", "Pionera", "Vértice", "Horizonte", "Granito", "Cobre Rojo",
    "Acero Central", "Ámbar", "Cardinal", "Lumbre"]
MUNIS = [("Puebla", "Puebla"), ("Puebla", "San Martín Texmelucan"), ("Puebla", "Cuautlancingo"), ("Puebla", "Huejotzingo"),
    ("Tlaxcala", "Apizaco"), ("Tlaxcala", "Tlaxcala"), ("Tlaxcala", "Xicohtzinco"), ("Puebla", "Amozoc")]
ESTRATOS = ["6 a 10 personas", "11 a 30 personas", "31 a 50 personas", "51 a 100 personas", "101 a 250 personas"]
GIRO_NAME = {"metal": "Metalmecánica", "estruct": "Metalmecánica", "auto": "Autopartes", "transp": "Transporte", "log": "Logística",
    "alm": "Almacén", "const": "Constructora / contratista", "taller": "Taller industrial", "mayor": "Comercializadora B2B"}

# state plan: (estado, count)
PLAN = [("nuevo", 70), ("revisado", 14), ("contactado", 22), ("respondio", 6), ("respuesta_positiva", 4), ("reunion", 3),
        ("propuesta", 3), ("cliente", 3), ("no_interesado", 5), ("sin_respuesta", 5), ("descartado", 7)]
states = [s for s, n in PLAN for _ in range(n)]
rnd.shuffle(states)

N = len(states)
keys = list(GIROS)
ids, rows = [], []
for i in range(N):
    k = keys[i % len(keys)]
    cid, clase, pref, sig = GIROS[k]
    lugar = LUGARES[i % len(LUGARES)]
    suffix = "" if i < len(LUGARES) else f" {['del Valle', 'Express', 'MX', 'Industrial'][i // len(LUGARES) % 4]}"
    nombre = f"{pref} {lugar}{suffix}".upper()
    slug = "".join(c for c in f"{pref}-{lugar}{suffix}".lower().replace("á", "a").replace("é", "e").replace("í", "i").replace("ó", "o").replace("ú", "u").replace(" ", "-") if c.isalnum() or c == "-")
    ent, muni = MUNIS[rnd.randrange(len(MUNIS))]
    web = rnd.random() < 0.7
    dom = f"{slug}.test"
    pid = f"D{100000 + i * 37}"
    rows.append(dict(id=pid, nombre=nombre, giro=GIRO_NAME[k], clase=clase, cid=cid, estrato=ESTRATOS[rnd.choice([1, 1, 2, 2, 3, 4, 0])],
        ent=ent, muni=muni, tel=f"222555{i:04d}" if rnd.random() < 0.9 else "", dom=dom if web else None,
        correo=(f"ventas@{dom}" if web and rnd.random() < 0.8 else (f"contacto@{slug}.test" if rnd.random() < 0.3 else "")),
        sig=sig, k=k, estado=states[i], fecha=f"20{rnd.randint(10, 23)}-{rnd.randint(1, 12):02d}"))
    ids.append(pid)

cols = "id,nombre,giro,clase_actividad,clase_id,rama_id,subrama_id,estrato,tipo_establecimiento,entidad,municipio,localidad,telefono,correo,sitio,dominio,fecha_alta,estado,source_website,website_checked_at,enriquecido_en,correo_web"
for r in rows:
    web = r["dom"] is not None
    checked = (today - timedelta(days=rnd.randint(1, 20))).isoformat() if web else None
    con.execute(f"INSERT INTO prospectos({cols}) VALUES ({','.join('?'*22)})", (
        r["id"], r["nombre"], r["giro"], r["clase"], r["cid"], r["cid"][:4], r["cid"][:5], r["estrato"], "Fijo", r["ent"], r["muni"], r["muni"],
        r["tel"], r["correo"], f"www.{r['dom']}" if web else "", r["dom"], r["fecha"], r["estado"], 1 if web else 0, checked, checked,
        f"ventas@{r['dom']}" if web and rnd.random() < 0.6 else ""))
    if web:
        n = rnd.randint(2, len(r["sig"]))
        for s in r["sig"][:n]:
            con.execute("INSERT INTO web_signals(prospect_id,signal_type,signal_text,signal_source_url) VALUES (?,?,?,?)",
                        (r["id"], s, TEXT[s], f"https://www.{r['dom']}/servicios"))
        con.execute("INSERT OR REPLACE INTO web_cache(domain,checked_at,status,pages_fetched,email,whatsapp) VALUES (?,?,?,?,?,?)",
                    (r["dom"], checked, "ok", 2, f"ventas@{r['dom']}", ""))

MSG = [
    "Hola, vi que {n} fabrica bajo pedido y atiende cotizaciones. Trabajo con talleres y plantas para ordenar el seguimiento de órdenes y costos. ¿Le interesa que le comparta un ejemplo?",
    "Buen día. Encontré el sitio de {n} y me llamó la atención su manejo de inventarios. Ayudo a pequeñas industrias a consolidar reportes. ¿Con quién puedo hablar de esto?",
    "Hola, soy consultor de datos en Puebla. Revisé lo que publica {n} sobre entregas y quisiera platicarle una idea concreta para dar seguimiento a cada ruta.",
]
REPLIES = ["Interesante, ¿me puede mandar un ejemplo?", "Estamos revisando presupuesto, escríbame la próxima semana.", "Sí, podemos agendar una llamada el jueves.",
           "Por ahora no tenemos necesidad, gracias."]
now = datetime.now()
for r in rows:
    st, pid = r["estado"], r["id"]
    if st in ("nuevo", "revisado", "descartado"):
        if st == "revisado":
            con.execute("INSERT INTO actividad(fecha,tipo,prospecto_id) VALUES (?,?,?)", (today.isoformat(), "investigado", pid))
        continue
    d0 = now - timedelta(days=rnd.randint(2, 40))
    msg = rnd.choice(MSG).format(n=r["nombre"].title())
    con.execute("INSERT INTO interactions(prospect_id,created_at,channel,interaction_type,sequence_step,template_variant,message_rendered,notes) VALUES (?,?,?,?,?,?,?,?)",
                (pid, d0.strftime("%Y-%m-%d %H:%M:%S"), "whatsapp" if r["tel"] else "correo", "contacto", 0, rnd.choice("AB"), msg, ""))
    con.execute("INSERT INTO actividad(fecha,tipo,prospecto_id) VALUES (?,?,?)", (d0.date().isoformat(), "contacto", pid))
    follow = None
    if st == "contactado":
        follow = (today + timedelta(days=rnd.randint(-5, 6))).isoformat()
    if st in ("respondio", "respuesta_positiva", "reunion", "propuesta", "cliente", "no_interesado"):
        txt = {"no_interesado": REPLIES[3], "respondio": REPLIES[1]}.get(st, rnd.choice(REPLIES[:1] + REPLIES[2:3]))
        d1 = d0 + timedelta(days=rnd.randint(1, 4))
        con.execute("INSERT INTO interactions(prospect_id,created_at,channel,interaction_type,response_type,notes) VALUES (?,?,?,?,?,?)",
                    (pid, d1.strftime("%Y-%m-%d %H:%M:%S"), "whatsapp", "respuesta", "positiva" if st != "no_interesado" else "negativa", txt))
    con.execute("UPDATE prospectos SET contactado_en=?,proximo_seguimiento=?,toques=1 WHERE id=?", (d0.date().isoformat(), follow, pid))

# CRM: accounts, contacts, opportunities for advanced states
NAMES = [("Mariana Torres Ibarra", "Gerente de operaciones"), ("Luis Fernando Cortés", "Director general"), ("Andrea Salgado", "Jefa de compras"),
         ("Ricardo Meneses", "Gerente de planta"), ("Paola Vega Ruiz", "Administración")]
OPP = {"respuesta_positiva": ("diagnostico", 28000), "reunion": ("diagnostico", 35000), "propuesta": ("propuesta", 62000), "cliente": ("ganado", 48000)}
ts = now.strftime("%Y-%m-%d %H:%M:%S")
cadj = 0
for r in rows:
    if r["estado"] not in OPP and r["estado"] != "respondio":
        continue
    con.execute("INSERT INTO accounts(id,denue_id,name,created_at) VALUES (?,?,?,?)", (f"acc-{r['id']}", r["id"], r["nombre"].title(), ts))
    nm, role = NAMES[cadj % len(NAMES)]; cadj += 1
    con.execute("INSERT INTO contacts(account_id,name,role,email,phone,notes) VALUES (?,?,?,?,?,?)",
                (f"acc-{r['id']}", nm, role, f"{nm.split()[0].lower()}@{r['dom'] or 'correo.test'}", r["tel"] or None, "Contacto ficticio de demostración"))
    if r["estado"] in OPP:
        stage, amt = OPP[r["estado"]]
        recv = amt * 100 if stage == "ganado" else 0
        con.execute("INSERT INTO opportunities(account_id,profile_id,title,stage,amount_cents,received_cents,currency,next_action,due_date,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
                    (f"acc-{r['id']}", "automatizacion", f"Tablero de seguimiento · {r['nombre'].title()}", stage, amt * 100, recv, "MXN",
                     None if stage == "ganado" else "Enviar propuesta ajustada", None if stage == "ganado" else (today + timedelta(days=rnd.randint(2, 15))).isoformat(), ts, ts))
ETAPA = {"contactado": 1, "sin_respuesta": 1, "respondio": 2, "no_interesado": 2, "respuesta_positiva": 3, "reunion": 4, "propuesta": 5, "cliente": 6}
for r in rows:
    if r["estado"] in ETAPA:
        con.execute("UPDATE prospectos SET etapa_max=? WHERE id=?", (ETAPA[r["estado"]], r["id"]))
con.execute("INSERT OR IGNORE INTO interaction_offers(interaction_id,profile_id) SELECT id,'automatizacion' FROM interactions")
con.commit()
print(len(rows), "prospectos ficticios")
