import json
import socket
import time
from collections import deque

import cv2
import mediapipe as mp


UDP_IP = "127.0.0.1"
UDP_PORT = 7777
CONFIDENCE = 0.8


class GestureBridge:
    def __init__(self) -> None:
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.hands = mp.solutions.hands.Hands(
            model_complexity=0,
            max_num_hands=2,
            min_detection_confidence=0.5,
            min_tracking_confidence=0.5,
        )
        self.area_history = deque(maxlen=6)
        self.y_history = deque(maxlen=5)
        self.guard_active = False
        self.last_jump_at = 0.0

    def send(self, gesture: str, confidence: float, player_id: int = 0) -> None:
        payload = {
            "gesture": gesture,
            "confidence": confidence,
            "timestamp": time.time(),
            "playerId": player_id,
        }
        self.socket.sendto(json.dumps(payload).encode("utf-8"), (UDP_IP, UDP_PORT))

    def classify(self, landmarks) -> None:
        xs = [point.x for point in landmarks.landmark]
        ys = [point.y for point in landmarks.landmark]
        zs = [point.z for point in landmarks.landmark]

        width = max(xs) - min(xs)
        height = max(ys) - min(ys)
        area = width * height
        avg_z = sum(zs) / len(zs)

        self.area_history.append((area, avg_z))
        if len(self.area_history) < self.area_history.maxlen:
            return

        first_area, first_z = self.area_history[0]
        area_delta = area - first_area
        z_delta = first_z - avg_z

        wrist = landmarks.landmark[0]
        tip = landmarks.landmark[8]
        pinky = landmarks.landmark[20]
        self.y_history.append(wrist.y)

        horizontal_swing = tip.x - pinky.x
        guard_height = abs(tip.y - wrist.y)
        now = time.time()

        if len(self.y_history) == self.y_history.maxlen:
            upward_motion = self.y_history[0] - self.y_history[-1]
            if upward_motion > 0.075 and now - self.last_jump_at > 0.7:
                self.send("jump", 0.84)
                self.last_jump_at = now
                return

        if area_delta > 0.03 and z_delta > 0.08:
            self.send("force_push", 0.93)
            return

        if guard_height < 0.08:
            if not self.guard_active:
                self.send("guard_start", CONFIDENCE)
                self.guard_active = True
            return

        if self.guard_active:
            self.send("guard_end", CONFIDENCE)
            self.guard_active = False

        if horizontal_swing < -0.12:
            self.send("slash_left", 0.9)
        elif horizontal_swing > 0.12:
            self.send("slash_right", 0.9)

    def run(self) -> None:
        capture = cv2.VideoCapture(0)
        while capture.isOpened():
            success, frame = capture.read()
            if not success:
                continue

            frame = cv2.flip(frame, 1)
            rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            results = self.hands.process(rgb)

            if results.multi_hand_landmarks:
                for hand_landmarks in results.multi_hand_landmarks:
                    self.classify(hand_landmarks)

            cv2.imshow("Gesture Bridge", frame)
            if cv2.waitKey(1) & 0xFF == 27:
                break

        capture.release()
        cv2.destroyAllWindows()


if __name__ == "__main__":
    GestureBridge().run()
