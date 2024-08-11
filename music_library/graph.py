class MusicLibraryGraph:
    def build_graph(self):
        graph = {}

        def add_edge(a, b, relation):
            if a not in graph:
                graph[a] = {}
            if b not in graph:
                graph[b] = {}
            if b not in graph[a]:
                graph[a][b] = {"strength": 0, "details": set()}
            if a not in graph[b]:
                graph[b][a] = {"strength": 0, "details": set()}
            graph[a][b]["strength"] += 1
            graph[a][b]["details"].add(relation)
            graph[b][a]["strength"] += 1
            graph[b][a]["details"].add(relation)

        # 遍历所有歌曲
        for song in self.songs.values():
            artist_uuids = [
                artist["uuid"]
                for artist in song.artists
                if artist["name"].lower() != "various artists"
            ]
            for i in range(len(artist_uuids)):
                for j in range(i + 1, len(artist_uuids)):
                    a, b = artist_uuids[i], artist_uuids[j]
                    add_edge(a, b, f"same song: {song.name} ({song.uuid})")

        # 遍历所有专辑
        for album in self.albums.values():
            album_artist_uuids = [
                artist.uuid
                for artist in album.album_artists
                if artist.name.lower() != "various artists"
            ]

            # 专辑艺术家之间的关系
            for i in range(len(album_artist_uuids)):
                for j in range(i + 1, len(album_artist_uuids)):
                    a, b = album_artist_uuids[i], album_artist_uuids[j]
                    add_edge(a, b, f"same album: {album.name} ({album.uuid})")

            # 专辑中的歌曲艺术家之间的关系
            for song in album.songs:
                song_artist_uuids = [
                    artist["uuid"]
                    for artist in song.artists
                    if artist["name"].lower() != "various artists"
                ]
                for i in range(len(song_artist_uuids)):
                    for j in range(i + 1, len(song_artist_uuids)):
                        a, b = song_artist_uuids[i], song_artist_uuids[j]
                        add_edge(a, b, f"same song: {song.name} ({song.uuid})")

                # 专辑艺术家与歌曲艺术家之间的关系
                for album_artist_uuid in album_artist_uuids:
                    for song_artist_uuid in song_artist_uuids:
                        if album_artist_uuid != song_artist_uuid:
                            add_edge(
                                album_artist_uuid,
                                song_artist_uuid,
                                f"album artist with song artist: {album.name} ({album.uuid})",
                            )
        return graph

    def find_relation(self, artist_uuid, graph):
        if artist_uuid not in self.artists:
            return {"nodes": [], "edges": []}

        nodes = [{"uuid": artist_uuid, "name": self.artists[artist_uuid].name}]
        edges = []
        visited = set([artist_uuid])

        def add_node_and_edge(source, target, data):
            if target not in visited:
                nodes.append({"uuid": target, "name": self.artists[target].name})
                visited.add(target)
            edges.append(
                {
                    "source": source,
                    "target": target,
                    "strength": data["strength"],
                    "details": list(data["details"]),
                }
            )

        for neighbor, data in graph.get(artist_uuid, {}).items():
            add_node_and_edge(artist_uuid, neighbor, data)
        for neighbor in graph:
            if artist_uuid in graph[neighbor]:
                data = graph[neighbor][artist_uuid]
                add_node_and_edge(neighbor, artist_uuid, data)

        return {"nodes": nodes, "edges": edges}

    def show_relation(self, artist_uuid, layer):
        graph = self.graph

        if artist_uuid == "show_all":
            nodes = []
            edges = []
            for a, neighbors in graph.items():
                if a not in self.artists:
                    continue
                nodes.append({"uuid": a, "name": self.artists[a].name})
                for b, data in neighbors.items():
                    if b not in self.artists:
                        continue
                    nodes.append({"uuid": b, "name": self.artists[b].name})
                    edges.append(
                        {
                            "source": a,
                            "target": b,
                            "strength": data["strength"],
                            "details": list(data["details"]),
                        }
                    )
            unique_nodes = {node["uuid"]: node for node in nodes}
            unique_edges = {
                tuple(sorted([edge["source"], edge["target"]])): edge for edge in edges
            }
            return {
                "nodes": list(unique_nodes.values()),
                "edges": list(unique_edges.values()),
            }

        if layer == 1:
            return self.find_relation(artist_uuid, graph)

        if layer < 1:
            return {
                "nodes": [
                    {"uuid": artist_uuid, "name": self.artists[artist_uuid].name}
                ],
                "edges": [],
            }

        all_nodes = [{"uuid": artist_uuid, "name": self.artists[artist_uuid].name}]
        all_edges = []
        visited = {artist_uuid}
        queue = [(artist_uuid, 0)]

        while queue:
            current_artist_uuid, current_layer = queue.pop(0)
            if current_layer >= layer:
                continue

            current_relations = self.find_relation(current_artist_uuid, graph)
            for node in current_relations["nodes"]:
                if node["uuid"] not in visited:
                    visited.add(node["uuid"])
                    all_nodes.append(node)
                    if current_layer + 1 < layer:
                        queue.append((node["uuid"], current_layer + 1))

            for edge in current_relations["edges"]:
                all_edges.append(edge)

        # 去重节点和边
        unique_nodes = {node["uuid"]: node for node in all_nodes}
        unique_edges = {
            tuple(sorted([edge["source"], edge["target"]])): edge for edge in all_edges
        }

        return {
            "nodes": list(unique_nodes.values()),
            "edges": list(unique_edges.values()),
        }
